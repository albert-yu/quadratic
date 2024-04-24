import { DashboardHeader } from '@/dashboard/components/DashboardHeader';
import { Empty } from '@/dashboard/components/Empty';
import { ConnectionsListComponent } from '@/dashboard/connections/components/ConnectionsListComponent';
import { apiClient } from '@/shared/api/apiClient';
import { ROUTES } from '@/shared/constants/routes';
import { AddOutlined } from '@mui/icons-material';
import { Button, useTheme } from '@mui/material';
import { ExclamationTriangleIcon } from '@radix-ui/react-icons';
import { Link, LoaderFunctionArgs, useLoaderData, useRouteError } from 'react-router-dom';

export const loader = async ({ request, params }: LoaderFunctionArgs) => {
  return await apiClient.getConnections();
};

export const Component = () => {
  const connections = useLoaderData() as Awaited<ReturnType<typeof loader>>;
  const theme = useTheme();

  return (
    <>
      <DashboardHeader
        title="Your connections"
        actions={
          <div style={{ display: 'flex', gap: theme.spacing(1) }}>
            <Button
              startIcon={<AddOutlined />}
              variant="contained"
              disableElevation
              component={Link}
              to={ROUTES.CONNECTIONS_CREATE}
            >
              Add Connection
            </Button>
          </div>
        }
      ></DashboardHeader>
      <ConnectionsListComponent connections={connections} />
    </>
  );
};

export const ErrorBoundary = () => {
  const error = useRouteError();

  // Maybe we log this to Sentry someday...
  console.error(error);
  return (
    <Empty
      title="Unexpected error"
      description="Something went wrong loading this file. If the error continues, contact us."
      Icon={ExclamationTriangleIcon}
      actions={
        <Button variant="contained" disableElevation component={Link} to="/">
          Go home
        </Button>
      }
      severity="error"
    />
  );
};
