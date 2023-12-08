import { debugShowMultiplayer } from '@/debugFlags';
import { grid } from '@/grid/controller/Grid';
import { sheets } from '@/grid/controller/Sheets';
import { pixiApp } from '@/gridGL/pixiApp/PixiApp';
import { pixiAppSettings } from '@/gridGL/pixiApp/PixiAppSettings';
import { Coordinate } from '@/gridGL/types/size';
import { SimpleMultiplayerUser } from '@/ui/menus/TopBar/useMultiplayerUsers';
import { User } from '@auth0/auth0-spa-js';
import { Rectangle } from 'pixi.js';
import { v4 as uuid } from 'uuid';

import { authClient } from '@/auth';
import { MULTIPLAYER_COLORS } from './multiplayerCursor/multiplayerColors';
import {
  Heartbeat,
  MessageTransaction,
  MessageUserUpdate,
  ReceiveMessages,
  ReceiveRoom,
  SendEnterRoom,
} from './multiplayerTypes';

const UPDATE_TIME = 1000 / 30;
const HEARTBEAT_TIME = 1000 * 15;
const RECONNECT_AFTER_ERROR_TIMEOUT = 1000 * 5;

export interface Player {
  firstName: string;
  lastName: string;
  sessionId: string;
  userId: string;
  sheetId?: string;
  cellEdit: { active: boolean; text: string; cursor: number };
  x?: number;
  y?: number;
  image: string;
  color: number;
  visible: boolean;
  selection?: { cursor: Coordinate; rectangle?: Rectangle };
}

export class Multiplayer {
  private websocket?: WebSocket;
  private state: 'not connected' | 'connecting' | 'connected' | 'waiting to reconnect';
  private sessionId;
  private room?: string;
  private user?: User;
  private jwt?: string | void;

  // messages pending a reconnect
  private waitingForConnection: { (value: unknown): void }[] = [];

  // queue of items waiting to be sent to the server on the next tick
  private userUpdate: MessageUserUpdate;
  private lastTime = 0;
  private lastHeartbeat = 0;

  // next player's color index
  private nextColor = 0;

  // users currently logged in to the room
  users: Map<string, Player> = new Map();

  constructor() {
    this.state = 'not connected';
    this.sessionId = uuid();
    this.userUpdate = { type: 'UserUpdate', session_id: this.sessionId, file_id: '', update: {} };
  }

  private async getJwt() {
    this.jwt = await authClient.getToken();
  }

  private async addJwtCookie(force: boolean = false) {
    if (force || !this.jwt) {
      await this.getJwt();

      if (this.jwt) {
        document.cookie = `jwt=${this.jwt}; path=/;`;
      }
    }
  }

  private async init() {
    if (this.state === 'connected') return;

    await this.addJwtCookie();

    return new Promise((resolve) => {
      if (this.state === 'connecting' || this.state === 'waiting to reconnect') {
        this.waitingForConnection.push(resolve);
        return;
      }

      this.state = 'connecting';
      this.websocket = new WebSocket(import.meta.env.VITE_QUADRATIC_MULTIPLAYER_URL);
      this.websocket.addEventListener('message', this.receiveMessage);

      this.websocket.addEventListener('close', this.reconnect);
      this.websocket.addEventListener('error', this.reconnect);

      this.websocket.addEventListener('open', () => {
        console.log('[Multiplayer] websocket connected.');
        this.state = 'connected';
        this.waitingForConnection.forEach((resolve) => resolve(0));
        resolve(0);
        this.waitingForConnection = [];
        this.lastHeartbeat = Date.now();
        window.addEventListener('change-sheet', this.sendChangeSheet);
      });
    });
  }

  private reconnect = () => {
    if (this.state === 'waiting to reconnect') return;
    console.log(`[Multiplayer] websocket closed. Reconnecting in ${RECONNECT_AFTER_ERROR_TIMEOUT / 1000}s...`);
    this.state = 'waiting to reconnect';
    setTimeout(async () => {
      this.state = 'not connected';
      await this.init();
      if (this.room) await this.enterFileRoom(this.room, this.user);
    }, RECONNECT_AFTER_ERROR_TIMEOUT);
  };

  // multiplayer for a file
  async enterFileRoom(file_id: string, user?: User) {
    // hack for same file different server
    // file_id = 'dde9887b-303c-491f-8863-0bfd047cce76';

    if (!user?.sub) throw new Error('User must be defined to enter a multiplayer room.');
    this.userUpdate.file_id = file_id;
    await this.init();
    this.user = user;
    if (this.room === file_id) return;
    this.room = file_id;
    const enterRoom: SendEnterRoom = {
      type: 'EnterRoom',
      session_id: this.sessionId,
      user_id: user.sub,
      file_id,
      sheet_id: sheets.sheet.id,
      selection: sheets.getMultiplayerSelection(),
      first_name: user.given_name ?? '',
      last_name: user.family_name ?? '',
      image: user.picture ?? '',
      cell_edit: {
        active: pixiAppSettings.input.show,
        text: pixiAppSettings.input.value ?? '',
        cursor: pixiAppSettings.input.cursor ?? 0,
      },
    };
    this.websocket!.send(JSON.stringify(enterRoom));
    if (debugShowMultiplayer) console.log(`[Multiplayer] Joined room ${file_id}.`);
  }

  // called by Update.ts
  async update() {
    if (this.state !== 'connected') return;
    const now = performance.now();
    if (now - this.lastTime < UPDATE_TIME) return;

    if (Object.keys(this.userUpdate.update).length > 0) {
      this.websocket!.send(JSON.stringify(this.userUpdate));
      this.userUpdate.update = {};
      this.lastHeartbeat = now;
    }
    this.lastTime = now;
    if (now - this.lastHeartbeat > HEARTBEAT_TIME) {
      const heartbeat: Heartbeat = {
        type: 'Heartbeat',
        session_id: this.sessionId,
        file_id: this.room!,
      };
      this.websocket!.send(JSON.stringify(heartbeat));
      if (debugShowMultiplayer) console.log('[Multiplayer] Sending heartbeat...');
      this.lastHeartbeat = now;
    }
  }

  // used to pre-populate useMultiplayerUsers.tsx
  getUsers(): SimpleMultiplayerUser[] {
    return Array.from(this.users.values()).map((player) => ({
      sessionId: player.sessionId,
      userId: player.userId,
      firstName: player.firstName,
      lastName: player.lastName,
      picture: player.image,
      color: player.color,
    }));
  }

  // whether a multiplayer user is already editing a cell
  cellIsBeingEdited(x: number, y: number, sheetId: string): boolean {
    for (const player of this.users.values()) {
      if (player.sheetId === sheetId && player.cellEdit.active && player.selection) {
        if (player.selection.cursor.x === x && player.selection.cursor.y === y) {
          return true;
        }
      }
    }
    return false;
  }

  //#region send messages
  //-------------------------

  private getUserUpdate(): MessageUserUpdate {
    if (!this.userUpdate) {
      this.userUpdate = {
        type: 'UserUpdate',
        session_id: this.sessionId,
        file_id: this.room!,
        update: {},
      };
    }
    return this.userUpdate;
  }

  async sendMouseMove(x?: number, y?: number) {
    const userUpdate = this.getUserUpdate().update;
    if (x === undefined || y === undefined) {
      userUpdate.visible = false;
    } else {
      userUpdate.x = x;
      userUpdate.y = y;
      userUpdate.visible = true;
    }
  }

  async sendSelection(selection: string) {
    const userUpdate = this.getUserUpdate().update;
    userUpdate.selection = selection;
  }

  sendChangeSheet = () => {
    const userUpdate = this.getUserUpdate().update;
    userUpdate.sheet_id = sheets.sheet.id;
  };

  sendCellEdit(text: string, cursor: number) {
    const userUpdate = this.getUserUpdate().update;
    userUpdate.cell_edit = {
      text,
      cursor,
      active: true,
    };
  }

  sendEndCellEdit() {
    const userUpdate = this.getUserUpdate().update;
    userUpdate.cell_edit = {
      text: '',
      cursor: 0,
      active: false,
    };
  }

  async sendTransaction(operations: string) {
    await this.init();
    const message: MessageTransaction = {
      type: 'Transaction',
      session_id: this.sessionId,
      file_id: this.room!,
      operations,
    };
    this.websocket!.send(JSON.stringify(message));
  }

  //#endregion

  //#region receive messages
  //-------------------------

  // updates the React hook to populate the Avatar list
  private receiveUsersInRoom(room: ReceiveRoom) {
    const players: SimpleMultiplayerUser[] = [];
    const remaining = new Set(this.users.keys());
    for (const user of room.users) {
      if (user.session_id !== this.sessionId) {
        let player = this.users.get(user.session_id);
        if (player) {
          player.firstName = user.first_name;
          player.lastName = user.last_name;
          player.image = user.image;
          player.sheetId = user.sheet_id;
          player.selection = user.selection ? JSON.parse(user.selection) : undefined;
          remaining.delete(user.session_id);
          if (debugShowMultiplayer) console.log(`[Multiplayer] Updated player ${user.first_name}.`);
        } else {
          player = {
            sessionId: user.session_id,
            userId: user.user_id,
            firstName: user.first_name,
            lastName: user.last_name,
            image: user.image,
            sheetId: user.sheet_id,
            selection: user.selection ? JSON.parse(user.selection) : undefined,
            cellEdit: user.cell_edit,
            x: 0,
            y: 0,
            color: this.nextColor,
            visible: false,
          };
          this.users.set(user.session_id, player);
          this.nextColor = (this.nextColor + 1) % MULTIPLAYER_COLORS.length;
          if (debugShowMultiplayer) console.log(`[Multiplayer] Player ${user.first_name} entered room.`);
        }
        players.push({
          sessionId: player.sessionId,
          userId: player.userId,
          firstName: player.firstName,
          lastName: player.lastName,
          picture: player.image,
          color: player.color,
        });
      }
    }
    remaining.forEach((sessionId) => {
      if (debugShowMultiplayer) console.log(`[Multiplayer] Player ${this.users.get(sessionId)?.firstName} left room.`);
      this.users.delete(sessionId);
    });
    window.dispatchEvent(new CustomEvent('multiplayer-update', { detail: players }));
    pixiApp.multiplayerCursor.dirty = true;
  }

  private receiveUserUpdate(data: MessageUserUpdate) {
    // this eventually will not be necessarily
    if (data.session_id === this.sessionId) return;
    const player = this.users.get(data.session_id);
    if (!player) {
      throw new Error("Expected Player to be defined before receiving a message of type 'MouseMove'");
    }
    if (data.file_id !== this.room) {
      throw new Error("Expected file_id to match room before receiving a message of type 'MouseMove'");
    }
    const update = data.update;

    if (update.x !== null && update.y !== null) {
      player.x = update.x;
      player.y = update.y;
      if (player.sheetId === sheets.sheet.id) {
        window.dispatchEvent(new CustomEvent('multiplayer-cursor'));
      }
    }

    if (update.visible !== undefined) {
      player.visible = update.visible;
    }

    if (update.sheet_id) {
      if (player.sheetId !== update.sheet_id) {
        player.sheetId = update.sheet_id;
        if (player.sheetId === sheets.sheet.id) {
          pixiApp.multiplayerCursor.dirty = true;
          window.dispatchEvent(new CustomEvent('multiplayer-cursor'));
        }
      }
    }

    if (update.selection) {
      player.selection = JSON.parse(update.selection);
      if (player.sheetId === sheets.sheet.id) {
        pixiApp.multiplayerCursor.dirty = true;
      }
    }

    if (update.cell_edit) {
      player.cellEdit = update.cell_edit;
      if (player.selection) {
        // hide the label if the player is editing the cell
        pixiApp.cellsSheets.showLabel(
          player.selection.cursor.x,
          player.selection.cursor.y,
          player.sheetId!,
          !player.cellEdit.active
        );
      }
      window.dispatchEvent(
        new CustomEvent('multiplayer-cell-edit', {
          detail: {
            ...update.cell_edit,
            playerColor: MULTIPLAYER_COLORS[player.color],
            sessionId: data.session_id,
            sheetId: player.sheetId,
            cell: player.selection?.cursor,
          },
        })
      );
      pixiApp.multiplayerCursor.dirty = true;
    }
  }

  private receiveTransaction(data: MessageTransaction) {
    // todo: this check should not be needed (eventually)
    if (data.session_id !== this.sessionId) {
      if (data.file_id !== this.room) {
        throw new Error("Expected file_id to match room before receiving a message of type 'Transaction'");
      }
      grid.multiplayerTransaction(data.operations);
    }
  }

  receiveMessage = (e: { data: string }) => {
    const data = JSON.parse(e.data) as ReceiveMessages;
    const { type } = data;
    if (type === 'UsersInRoom') {
      this.receiveUsersInRoom(data);
    } else if (type === 'UserUpdate') {
      this.receiveUserUpdate(data);
    } else if (type === 'Transaction') {
      this.receiveTransaction(data);
    } else if (type !== 'Empty') {
      console.warn(`Unknown message type: ${type}`);
    }
  };
}

export const multiplayer = new Multiplayer();
