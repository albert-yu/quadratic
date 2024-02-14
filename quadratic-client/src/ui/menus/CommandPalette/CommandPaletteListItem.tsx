import { ListItem, ListItemButton, ListItemIcon, ListItemSecondaryAction, ListItemText } from '@mui/material';
import fuzzysort from 'fuzzysort';
import mixpanel from 'mixpanel-browser';
import { ReactElement } from 'react';

// Props generated in the root CommandPalette and passed to every CommandPaletteListItem
export interface CommandPaletteListItemSharedProps {
  closeCommandPalette: Function;
  fuzzysortResult?: Fuzzysort.Result;
  label: string;
  listItemIndex: number;
  selectedListItemIndex: number;
  addKeywords?: string;
}

// Contextual props added to each individual <CommandPaletteListItem>
interface CommandPaletteListItemUniqueProps {
  action: Function;
  disabled?: boolean;
  icon?: ReactElement;
  shortcut?: string;
  shortcutModifiers?: Array<string> | string;
  keywords?: Array<string> | string;
}

// All props this component needs
interface CommandPaletteListItemProps extends CommandPaletteListItemSharedProps, CommandPaletteListItemUniqueProps {}

export const CommandPaletteListItem = (props: CommandPaletteListItemProps) => {
  const {
    selectedListItemIndex,
    closeCommandPalette,
    listItemIndex,
    action,
    disabled,
    label,
    shortcut,
    shortcutModifiers,
    icon,
    fuzzysortResult,
    addKeywords,
  } = props;

  let displayText: (string | JSX.Element)[] | null | string = label;

  // remove any keywords from the displayed search result
  if (addKeywords && fuzzysortResult?.target) {
    fuzzysort.highlight({ ...fuzzysortResult, target: fuzzysortResult.target.replace(addKeywords, '') }, (m, i) => (
      <b key={i}>{m}</b>
    ));
  } else if (fuzzysortResult) {
    displayText = fuzzysort.highlight(fuzzysortResult, (m, i) => <b key={i}>{m}</b>);
  }

  return (
    <ListItem disablePadding key={label}>
      <ListItemButton
        // FYI: this is used to animate scroll through list items and trigger click on ENTER
        data-command-bar-list-item-index={listItemIndex}
        disabled={disabled}
        onClick={() => {
          mixpanel.track('[CommandPalette].run', { label: label });
          action();
          closeCommandPalette();
        }}
        selected={listItemIndex === selectedListItemIndex}
      >
        {icon && <ListItemIcon>{icon}</ListItemIcon>}
        <ListItemText primary={displayText} inset={icon ? false : true} />

        {shortcut && (
          <ListItemSecondaryAction style={{ fontSize: '14px', opacity: '.5' }}>
            {shortcutModifiers ? shortcutModifiers : ''}
            {shortcut}
          </ListItemSecondaryAction>
        )}
      </ListItemButton>
    </ListItem>
  );
};
