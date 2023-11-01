import { AccessSchema, UserRoleFileSchema, UserRoleTeam, UserRoleTeamSchema } from '../../../src/permissions';

const { TEAM_EDIT, TEAM_DELETE, TEAM_BILLING_EDIT, TEAM_VIEW, FILE_VIEW, FILE_EDIT, FILE_DELETE } = AccessSchema.enum;

export const getTeamAccess = (role: UserRoleTeam) => {
  const { OWNER, EDITOR, VIEWER } = UserRoleFileSchema.enum;
  switch (role) {
    case OWNER:
      return [TEAM_EDIT, TEAM_VIEW, TEAM_DELETE, TEAM_BILLING_EDIT];
    case EDITOR:
      return [TEAM_EDIT, TEAM_VIEW];
    case VIEWER:
      return [TEAM_VIEW];
    default:
      return [];
  }
};

export const getFileAccess = (userRole: UserRoleTeam, context: 'FILE' | 'TEAM') => {
  const { OWNER, EDITOR, VIEWER } = UserRoleTeamSchema.enum;
  switch (userRole) {
    case OWNER:
      return [FILE_EDIT, FILE_VIEW, FILE_DELETE];
    case EDITOR:
      return [FILE_EDIT, FILE_VIEW, ...(context === 'TEAM' ? [FILE_DELETE] : [])];
    case VIEWER:
      return [FILE_VIEW];
    default:
      return [];
  }
};
