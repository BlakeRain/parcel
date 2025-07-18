import { html } from "htm/preact";
import { useEffect } from "preact/hooks";
import { StateMode, useState } from "../state";

const STATE_COLORS = {
  idle: "bg-neutral-100 dark:bg-slate-800/50 text-neutral-500 dark:text-slate-500",
  active:
    "bg-neutral-200 dark:bg-slate-800/75 text-neutral-600 dark:text-slate-400",
  complete:
    "bg-green-200 dark:bg-green-900/50 text-green-600 dark:text-green-500",
  error: "bg-red-200 dark:bg-red-900/50 text-red-400 dark:text-red-600",
};

const DropZone = () => {
  const { state, dispatch } = useState();

  let colors: string, icon: string, title: string, subtitle: string;
  switch (state.mode) {
    case StateMode.Preparing:
      icon = state.dragIcon;
      title = state.dragHint || "Drop files";
      subtitle = "Drag and drop your files here or click to select files";
      colors =
        state.dragFiles.length > 0 ? STATE_COLORS.active : STATE_COLORS.idle;
      break;
    case StateMode.Uploading:
      icon = "icon-loader-circle animate-rotate";
      title = "Uploading files";
      subtitle = "Please wait while we upload your files";
      colors = STATE_COLORS.active;
      break;
    case StateMode.Error:
      icon = "icon-octagon-alert";
      title = "Failed to upload files";
      subtitle = "There was an error uploading your files";
      colors = STATE_COLORS.error;
      break;
    case StateMode.Aborted:
      icon = "icon-x-circle";
      title = "Upload aborted";
      subtitle = "The upload was aborted";
      colors = STATE_COLORS.error;
      break;
    case StateMode.Complete:
      icon = "icon-badge-circle";
      title = "Upload complete";
      subtitle = "Your files have been uploaded successfully";
      colors = STATE_COLORS.complete;
      break;
  }

  useEffect(() => {
    const onDragOver = (event: DragEvent) => {
      dispatch({ type: "dragover", event });
    };

    const onDragLeave = () => {
      dispatch({ type: "dragleave" });
    };

    const onDragDrop = (event: DragEvent) => {
      event.preventDefault();
      dispatch({ type: "drop", event });
    };

    document.body.addEventListener("dragover", onDragOver);
    document.body.addEventListener("dragleave", onDragLeave);
    document.body.addEventListener("drop", onDragDrop);

    return () => {
      document.body.removeEventListener("dragover", onDragOver);
      document.body.removeEventListener("dragleave", onDragLeave);
      document.body.removeEventListener("drop", onDragDrop);
    };
  });

  const onClick = () => {
    if (!state.upload) {
      const input = document.createElement("input");
      input.type = "file";
      input.multiple = true;

      input.addEventListener("change", (event) => {
        const files = [...(event.target as HTMLInputElement).files];

        dispatch({
          type: "add",
          files,
        });
      });

      input.click();
    }
  };

  const classes =
    "transition-colors cursor-pointer border border-gray-300 dark:border-slate-600 rounded-md p-8 flex flex-col gap-4 " +
    colors;

  return html`
    <div class=${classes} onclick=${onClick}>
      <h1 class="text-3xl font-bold text-center select-none gap-2">
        <span class="${icon}"></span>
        <span> </span>
        <span>${title}</span>
      </h1>
      <h2 class="text-xl font-semibold text-center select-none">${subtitle}</h2>
    </div>
  `;
};

export default DropZone;
