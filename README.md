![Quadratic Tests](https://github.com/quadratichq/quadratic/actions/workflows/main.yml/badge.svg) ![License ELv2](https://user-images.githubusercontent.com/3479421/162047443-5469b5a7-43e9-4c23-a2fa-3f9e5b2ecfaf.svg)


![quadraticlogo4 1](https://user-images.githubusercontent.com/3479421/162037216-2fea1620-2310-4cfa-96fb-31299195e3a9.png)

![quardatic icon small](https://user-images.githubusercontent.com/3479421/162039117-02f85f2c-e382-4ed8-ac39-64efab17a144.svg)  **_The Data Science Spreadsheet_**
----

Infinite data grid with Python, JavaScript, and SQL built-in. Data Connectors to pull in your data.

Take your data and do something useful with it as quickly and easily as possible!

![Screen Shot 2022-04-07 at 4 15 52 PM](https://user-images.githubusercontent.com/3479421/162328478-198f27d1-4ab8-4334-8420-b082e68edefc.png)

# What is a Data Science Spreadsheet?

Quadratic is a Web-based spreadsheet application that runs both in the browser and as a standalone application (via Electron). 

Our goal is to build the easiest way to pull your data from wherever it happens to be (SaaS, Database, CSV, API, etc) and then allow you to work with that data using the most popular data science tools today (Python, Pandas, SQL, JS, Excel Formulas, etc). 

Quadratic has no environment to configure. Our spreadsheet runs entirely in the browser with no backend service. This makes our grids completely portable and very easy to share.

## What can I do with Quadratic?
- Build internal tools
- Build dashboards
- Quickly mix data from different sources
- Explore your data for new insights

# Development Progress and Roadmap

_Quadratic is in ALPHA. For now, we do not recommend relying on Quadratic._

- [x] WebGL Grid (pinch and zoom grid)
- [x] Open and Save files locally
- [x] Python
- [x] Pandas Support
- [ ] Database Connection Support (issue [#35](https://github.com/quadratichq/quadratic/issues/35))
- [ ] SQL Support (issue [#34](https://github.com/quadratichq/quadratic/issues/34))
- [ ] Undo / Redo (issue [#42](https://github.com/quadratichq/quadratic/issues/42))
- [ ] Cell Formatting (issue [#44](https://github.com/quadratichq/quadratic/issues/44))
- [ ] Import CSV
- [ ] JS Support

Notice a bug? Submit a Github Issue!

Want to learn more about how Quadratic works? Read our [How Quadratic Works](./docs/how_quadratic_works.md) doc.

# Getting Started

## Online Demo

We have a hosted version of the `main` branch available online. Try it out!

https://early.quadratic.to

## Run Quadratic Locally

Install Dependencies `npm install`

Run Web `npm start`

Run Electron `npm run dev`

# Documentation

Read the docs on [docs.quadratic.to](https://docs.quadratic.to)
- [Quick Start Guide](https://docs.quadratic.to/quick-start)
- [Python Cell Reference](https://docs.quadratic.to/reference/python-cell-reference)
- [Pandas DataFrames in Quadratic](https://docs.quadratic.to/reference/python-cell-reference/pandas-dataframe)
- [Development Updates](https://docs.quadratic.to/development-updates)

# Examples

Example files are located in the `examples` folder in this repo.

# License
Quadratic is licensed under the Elastic License 2.0 (ELv2).
