use arrow::{
    array::{ArrayRef, RecordBatch, StringArray},
    datatypes::*,
};
use bytes::Bytes;
use futures_util::Future;
use parquet::arrow::ArrowWriter;
use rayon::prelude::*;
use sqlx::{Column, Row};
use std::sync::Arc;
use tokio::time::Instant;

use self::{mysql_connection::MysqlConnection, postgres_connection::PostgresConnection};

pub mod mysql_connection;
pub mod postgres_connection;

pub enum SqlConnection<'a> {
    Postgres(PostgresConnection<'a>),
    Mysql(MysqlConnection),
}

pub trait Connection {
    type Pool;
    type Row: Row;
    type Column: Column;

    /// Create a connection string from the connection parameters
    fn connection_string(&self) -> String;

    fn connect(&self) -> impl Future<Output = Result<Self::Pool, sqlx::Error>>;

    /// Generically query a database
    fn query(
        &self,
        pool: Self::Pool,
        sql: &str,
    ) -> impl Future<Output = Result<Vec<Self::Row>, sqlx::Error>>;

    /// Convert a database-specific column to an Arrow type
    fn to_arrow(
        row: &Self::Row,
        column: &<<Self::Row as sqlx::Row>::Database as sqlx::Database>::Column,
        index: usize,
    ) -> Option<String>;

    /// Default implementation of converting a vec of rows to a Parquet byte array
    ///
    /// This should work over any row/colmn SQLx vec
    fn to_parquet(data: Vec<Self::Row>) -> Bytes
    where
        Self::Row: Row,
        Self::Column: Column,
    {
        let start = Instant::now();
        let fields = data[0]
            .columns()
            .iter()
            .map(|col| Field::new(col.name().to_string(), DataType::Utf8, true))
            .collect::<Vec<Field>>();

        let col_count = fields.len();
        let schema = Schema::new(fields);

        let duration = start.elapsed();
        tracing::info!("Setup: {:?}", duration);
        let start = Instant::now();

        // transpose columns to rows, converting to Arrow types
        let mut transposed = vec![vec![]; col_count];

        data.iter().for_each(|row| {
            row.columns()
                .iter()
                .enumerate()
                .for_each(|(col_index, col)| {
                    let value = Self::to_arrow(row, col, col_index).unwrap_or("".into());
                    transposed[col_index].push(value);
                });
        });

        let duration = start.elapsed();
        tracing::info!("Transposing: {:?}", duration);
        let start = Instant::now();

        let file = Vec::new();
        let mut writer = ArrowWriter::try_new(file, Arc::new(schema.clone()), None).unwrap();

        let cols = transposed
            .into_iter()
            .map(|col| Arc::new(StringArray::from_iter_values(col)) as ArrayRef)
            .collect::<Vec<ArrayRef>>();

        let duration = start.elapsed();
        tracing::info!("Collecting: {:?}", duration);
        let start = Instant::now();

        writer
            .write(&RecordBatch::try_new(Arc::new(schema.clone()), cols).unwrap())
            .unwrap();

        let duration = start.elapsed();
        tracing::info!("Writing bytes: {:?}", duration);

        writer.into_inner().unwrap().into()
    }
}
