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

  static async fromEvent(event: DragEvent) {
    if (event.dataTransfer.items) {
      const items = [...event.dataTransfer.items].filter(
        (item) => item.kind === "file",
      );

      let files = [];
      for (let item of items) {
        const entry = item.webkitGetAsEntry();
        await scanFiles(entry, files);
      }

      return files;
    }

    return [];
  }

  static fromList(files: FileList) {
    return [...files].map((file) => new FileInfo(file));
  }
}

async function scanFiles(entry: FileSystemEntry, files: FileInfo[]) {
  if (entry.isDirectory) {
    const promise = new Promise((resolve, reject) => {
      (entry as FileSystemDirectoryEntry)
        .createReader()
        .readEntries((entries) => {
          const f = entries.map((entry) => scanFiles(entry, files));
          Promise.all(f).then(resolve, reject);
        }, reject);
    });

    await promise;
  } else if (entry.isFile) {
    const promise: Promise<File> = new Promise((resolve, reject) => {
      (entry as FileSystemFileEntry).file(resolve, reject);
    });

    const file = await promise;
    files.push(new FileInfo(file));
  }
}
