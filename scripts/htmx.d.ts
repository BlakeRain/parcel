declare module "htmx" {
  export interface Htmx {
    remove(element: HTMLElement): void;
  }
}

declare const htmx: import("htmx").Htmx;
