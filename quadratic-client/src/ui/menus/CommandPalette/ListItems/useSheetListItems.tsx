import { events } from '@/events/events';
import ArticleOutlinedIcon from '@mui/icons-material/ArticleOutlined';
import { useEffect, useState } from 'react';
import { sheets } from '../../../../grid/controller/Sheets';
import { CommandPaletteListItem } from '../CommandPaletteListItem';
import { Commands } from '../getCommandPaletteListItems';

export const useSheetListItems = (): Commands[] => {
  const [commands, setCommands] = useState<Commands[]>([]);

  const updateCommands = () => {
    setCommands(
      sheets.getSheetListItems().map((item) => ({
        label: `Sheet: ${item.name}`,
        Component: (props: any) => (
          <CommandPaletteListItem {...props} icon={<ArticleOutlinedIcon />} action={() => (sheets.current = item.id)} />
        ),
      }))
    );
  };

  useEffect(() => {
    events.on('changeSheet', updateCommands);
    return () => {
      events.off('changeSheet', updateCommands);
    }
  }, []);

  useEffect(() => updateCommands(), []);

  return commands;
};
