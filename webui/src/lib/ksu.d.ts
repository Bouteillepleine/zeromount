interface KsuNativeBridge {
  exec(cmd: string, options: string, callbackName: string): void;
  spawn(cmd: string, argsJson: string, optionsJson: string, callbackName: string): void;
  fullScreen(isFullScreen: boolean): void;
  enableEdgeToEdge(enable: boolean): void;
  toast(msg: string): void;
  moduleInfo(): string;
  listPackages(type: string): string;
  getPackagesInfo(packagesJson: string): string;
  exit(): void;
}

declare global {
  var ksu: KsuNativeBridge | undefined;

  interface Window {
    [key: `exec_callback_${string}`]: ((...args: any[]) => void) | undefined;
    [key: `spawn_callback_${string}`]: any;
  }
}

export type { KsuNativeBridge };
