const MIME_INFO = {
  "application/pdf": { icon: "file-type", hint: "PDF document" },
  "application/msword": { icon: "file-word", hint: "Microsoft Word document" },
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document": {
    icon: "file-word",
    hint: "Microsoft Word document",
  },
  "application/vnd.ms-excel": {
    icon: "file-spreadsheet",
    hint: "Microsoft Excel spreadsheet",
  },
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet": {
    icon: "file-spreadsheet",
    hint: "Microsoft Excel spreadsheet",
  },
  "application/vnd.ms-powerpoint": {
    icon: "file-video",
    hint: "Microsoft PowerPoint presentation",
  },
  "application/vnd.openxmlformats-officedocument.presentationml.presentation": {
    icon: "file-video",
    hint: "Microsoft PowerPoint presentation",
  },
  "application/zip": { icon: "file-archive", hint: "ZIP archive" },
  "application/x-tar": { icon: "file-archive", hint: "TAR archive" },
  "application/x-gzip": { icon: "file-archive", hint: "GZIP archive" },
  "application/x-bzip2": { icon: "file-archive", hint: "BZIP2 archive" },
  "application/x-diskcopy": { icon: "file-archive", hint: "Disk Copy archive" },
  "application/x-7z-compressed": {
    icon: "file-archive",
    hint: "7-Zip archive",
  },
  "application/x-rar-compressed": { icon: "file-archive", hint: "RAR archive" },
  "application/x-xz": { icon: "file-archive", hint: "XZ archive" },
  "application/x-zip-compressed": { icon: "file-archive", hint: "ZIP archive" },
  "text/plain": { icon: "file-text", hint: "Text file" },
  "text/html": { icon: "file-code", hint: "HTML document" },
  "text/css": { icon: "file-code", hint: "CSS stylesheet" },
  "text/javascript": { icon: "file-code", hint: "JavaScript file" },
  "application/json": { icon: "file-json", hint: "JSON document" },
  "application/xml": { icon: "file-code", hint: "XML document" },
  "application/ld+json": { icon: "file-json", hint: "JSON-LD document" },
  "audio/mpeg": { icon: "file-audio", hint: "MP3 audio" },
  "audio/ogg": { icon: "file-audio", hint: "OGG audio" },
  "audio/wav": { icon: "file-audio", hint: "WAV audio" },
  "audio/webm": { icon: "file-audio", hint: "WebM audio" },
  "audio/flac": { icon: "file-audio", hint: "FLAC audio" },
  "audio/aac": { icon: "file-audio", hint: "AAC audio" },
  "image/gif": { icon: "file-image", hint: "GIF image" },
  "image/jpeg": { icon: "file-image", hint: "JPEG image" },
  "image/png": { icon: "file-image", hint: "PNG image" },
  "image/webp": { icon: "file-image", hint: "WebP image" },
  "image/svg+xml": { icon: "file-image", hint: "SVG image" },
  "video/mp4": { icon: "file-video-2", hint: "MP4 video" },
  "video/ogg": { icon: "file-video-2", hint: "OGG video" },
  "video/webm": { icon: "file-video-2", hint: "WebM video" },
  "video/x-matroska": { icon: "file-video-2", hint: "Matroska video" },
  "video/quicktime": { icon: "file-video-2", hint: "QuickTime video" },
  "video/x-msvideo": { icon: "file-video-2", hint: "AVI video" },
  "video/x-ms-wmv": { icon: "file-video-2", hint: "Windows Media Video" },
};

export default MIME_INFO;