import { useRootRouteLoaderData } from '@/router';
import { getUpdateEducationAction } from '@/routes/education';
import { Button } from '@/shadcn/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/shadcn/ui/dialog';
import { SchoolOutlined } from '@mui/icons-material';
import { useEffect, useState } from 'react';
import { useFetcher } from 'react-router-dom';

export function EducationDialog() {
  const [open, onOpenChange] = useState<boolean>(false);
  const fetcher = useFetcher();
  const { loggedInUser } = useRootRouteLoaderData();

  // TODO: localstorage get when this last fetched so we can throttle every few mins

  const handleClose = () => {
    onOpenChange(false);
    // const { data, options } = getUpdateEducationAction('NOT_ENROLLED');
    // fetcher.submit(data, options);
  };

  useEffect(() => {
    if (fetcher.state === 'idle' && !fetcher.data) {
      const email = loggedInUser?.email;
      if (typeof email === 'undefined') {
        // sentry error
        return;
      }

      const { data, options } = getUpdateEducationAction({ email });
      console.log('Fired mount fetch', data, options);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader className="text-center sm:text-center">
          <div className="flex flex-col items-center py-4">
            <SchoolOutlined sx={{ fontSize: '64px' }} className="text-primary" />
          </div>
          <DialogTitle>Enrolled in Quadratic for Education</DialogTitle>
          <DialogDescription>
            You have an educational email address which qualifies you for{' '}
            <a href="TODO:" target="_blank" rel="noreferrer" className="underline hover:text-primary">
              the education plan
            </a>{' '}
            where students, teachers, and researchers get free access.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter className="justify-center text-center sm:justify-center">
          <Button onClick={handleClose}>Ok, thanks</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
