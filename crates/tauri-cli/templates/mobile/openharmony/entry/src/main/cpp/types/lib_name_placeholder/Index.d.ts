export const ohosInit: () => void;
export const registerXcomponent: (arg: object) => void;
export const registerWebview: (callback: (script: string) => void) => void;
export const resolveRequest: (id: string, url: string) => string | null;
export const onIpcMessage: (id: string, msg: string) => void;
