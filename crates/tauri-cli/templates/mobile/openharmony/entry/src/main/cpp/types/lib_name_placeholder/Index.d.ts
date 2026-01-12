export const ohosInit: () => void;
export const onWindowStageCreate: () => void;
export const onWindowStageDestroy: () => void;
export const onWindowStageFocus: () => void;
export const onWindowStageBlur: () => void;
export const registerWebview: (callback: (script: string) => void) => void;
export const resolveRequest: (id: string, url: string) => Uint8Array | null;
export const onIpcMessage: (id: string, msg: string) => void;
