import MIME_INFO from "./mime.js";

export class DragFile {
  public type: string;

  constructor(type: string) {
    this.type = type;
  }

  get mime_info() {
    return MIME_INFO[this.type];
  }

  get icon() {
    const info = this.mime_info;
    if (info && info.icon) {
      return `icon-${info.icon}`;
    }

    return "icon-file";
  }

  get hint() {
    const info = this.mime_info;
    if (info && info.hint) {
      return info.hint;
    }

    return null;
  }

  static fromEvent(event: DragEvent) {
    if (event.dataTransfer.items) {
      return [...event.dataTransfer.items]
        .filter((item) => item.kind === "file")
        .map((item) => new DragFile(item.type));
    }

    return [];
  }
}

export class FileInfo {
  public file: File;

  constructor(file: File) {
    this.file = file;
  }

  get name() {
    return this.file.name;
  }

  get size() {
    return this.file.size;
  }

  get type() {
    return this.file.type;
  }

  get mime_info() {
    return MIME_INFO[this.type];
  }

  get icon() {
    const info = this.mime_info;
    if (info && info.icon) {
      return `icon-${info.icon}`;
    }

    return "icon-file";
  }

  static fromEvent(event: DragEvent) {
    if (event.dataTransfer.items) {
      return [...event.dataTransfer.items]
        .filter((item) => item.kind === "file")
        .map((item) => new FileInfo(item.getAsFile()));
    }

    return [];
  }

  static fromList(files: FileList) {
    return [...files].map((file) => new FileInfo(file));
  }
}
