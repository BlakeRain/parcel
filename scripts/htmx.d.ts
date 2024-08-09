declare module "htmx" {
  export interface Htmx {
    ajax(method: string, url: string, options?: any): Promise<any>;
    remove(element: HTMLElement): void;
    trigger(selector: string, event: string): void;
  }
}

declare const htmx: import("htmx").Htmx;
