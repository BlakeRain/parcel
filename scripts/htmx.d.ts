declare module "htmx" {
  export interface Htmx {
    remove(element: HTMLElement): void;
    trigger(selector: string, event: string): void;
  }
}

declare const htmx: import("htmx").Htmx;
