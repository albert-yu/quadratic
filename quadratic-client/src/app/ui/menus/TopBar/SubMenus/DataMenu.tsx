import { downloadSelectionAsCsvAction, isAvailableBecauseFileLocationIsAccessibleAndWriteable } from '@/app/actions';
import { editorInteractionStateAtom } from '@/app/atoms/editorInteractionStateAtom';
import { KeyboardSymbols } from '@/app/helpers/keyboardSymbols';
import { useFileContext } from '@/app/ui/components/FileProvider';
import { DataIcon } from '@/app/ui/icons';
import { useRootRouteLoaderData } from '@/routes/_root';
import { useFileRouteLoaderData } from '@/routes/file.$uuid';
import { useGlobalSnackbar } from '@/shared/components/GlobalSnackbarProvider';
import { CSV_IMPORT_MESSAGE, PARQUET_IMPORT_MESSAGE } from '@/shared/constants/appConstants';
import { Menu, MenuDivider, MenuItem } from '@szhsin/react-menu';
import '@szhsin/react-menu/dist/index.css';
import { useSetRecoilState } from 'recoil';
import { MenuLineItem } from '../MenuLineItem';
import { TopBarMenuItem } from '../TopBarMenuItem';

export const DataMenu = () => {
  const setEditorInteractionState = useSetRecoilState(editorInteractionStateAtom);
  const { addGlobalSnackbar } = useGlobalSnackbar();
  const { name: fileName } = useFileContext();
  const { isAuthenticated } = useRootRouteLoaderData();
  const {
    userMakingRequest: { filePermissions, fileTeamPrivacy, teamPermissions },
  } = useFileRouteLoaderData();
  const showManageConnections = isAvailableBecauseFileLocationIsAccessibleAndWriteable({
    isAuthenticated,
    filePermissions,
    fileTeamPrivacy,
    teamPermissions,
  });

  return (
    <>
      <Menu
        menuButton={({ open }) => (
          <TopBarMenuItem title="Data" open={open}>
            <DataIcon fontSize="small" />
          </TopBarMenuItem>
        )}
      >
        <MenuItem
          onClick={() => {
            addGlobalSnackbar(CSV_IMPORT_MESSAGE);
          }}
        >
          <MenuLineItem primary="Import CSV" />
        </MenuItem>
        <MenuItem
          onClick={() => {
            addGlobalSnackbar(PARQUET_IMPORT_MESSAGE);
          }}
        >
          <MenuLineItem primary="Import Parquet" />
        </MenuItem>
        <MenuDivider />
        <MenuItem
          onClick={() => {
            downloadSelectionAsCsvAction.run({ fileName });
          }}
        >
          <MenuLineItem
            primary={downloadSelectionAsCsvAction.label}
            secondary={KeyboardSymbols.Command + KeyboardSymbols.Shift + 'E'}
          />
        </MenuItem>
        {showManageConnections && (
          <>
            <MenuDivider />
            <MenuItem onClick={() => setEditorInteractionState((prev) => ({ ...prev, showConnectionsMenu: true }))}>
              <MenuLineItem primary="Manage connections" />
            </MenuItem>
          </>
        )}
      </Menu>
    </>
  );
};
