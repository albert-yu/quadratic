import { CommandPaletteListItemSharedProps } from '../CommandPaletteListItem';
import { CommandPaletteListItem } from '../CommandPaletteListItem';
import { NoteAddOutlined, UploadFileOutlined } from '@mui/icons-material';
import { SaveFileOutlined } from '../../../icons';
import { KeyboardSymbols } from '../../../../helpers/keyboardSymbols';
import { useRecoilState } from 'recoil';
import { editorInteractionStateAtom } from '../../../../atoms/editorInteractionStateAtom';
import { useContext } from 'react';
import { AppContext } from '../../../QuadraticUI';

const ListItems = [
  {
    label: 'File: New',
    Component: (props: CommandPaletteListItemSharedProps) => {
      const {
        localFiles: { newFile },
      } = useContext(AppContext);
      return <CommandPaletteListItem {...props} icon={<NoteAddOutlined />} action={newFile} />;
    },
  },
  {
    label: 'File: Save local copy',
    Component: (props: CommandPaletteListItemSharedProps) => {
      const {
        localFiles: { downloadQuadraticFile },
      } = useContext(AppContext);
      return <CommandPaletteListItem {...props} icon={<SaveFileOutlined />} action={() => downloadQuadraticFile()} />;
    },
  },
  {
    label: 'File: Open…',
    Component: (props: CommandPaletteListItemSharedProps) => {
      const [editorInteractionState, setEditorInteractionState] = useRecoilState(editorInteractionStateAtom);
      return (
        <CommandPaletteListItem
          {...props}
          icon={<UploadFileOutlined />}
          shortcut="O"
          shortcutModifiers={[KeyboardSymbols.Command]}
          action={() => {
            setEditorInteractionState({
              ...editorInteractionState,
              showFileMenu: true,
            });
          }}
        />
      );
    },
  },
];

export default ListItems;
