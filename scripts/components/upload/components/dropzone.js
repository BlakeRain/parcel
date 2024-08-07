import { html } from "../../../shared.js";
import { useEffect } from "preact/hooks";
import { useState } from "../state.js";

const DropZone = () => {
  const { state, dispatch } = useState();

  const colors =
    state.dragFiles.length > 0 || state.upload
      ? "bg-neutral-200 text-neutral-600 dark:text-neutral-400"
      : state.complete
        ? "bg-green-200 text-green-400 dark:text-green-600"
        : "bg-neutral-100 text-neutral-400 dark:text-neutral-600";

  const classes =
    "border dark:bg-slate-800/50 border-gray-300 dark:border-slate-600 rounded-md p-8 flex flex-col gap-4 " +
    colors;

  let icon, title, subtitle;
  if (state.upload) {
    icon = "icon-loader-circle animate-rotate";
    title = "Uploading files";
    subtitle = "Please wait while we upload your files";
  } else if (state.error) {
    icon = "icon-octagon-alert";
    title = "Failed to upload files";
    subtitle = "Please try again";
  } else if (state.complete) {
    icon = "icon-badge-circle";
    title = "Upload complete";
    subtitle = "Your files have been uploaded successfully";
  } else {
    icon = state.dragIcon;
    title = state.dragHint || "Drop files";
    subtitle = "Drag and drop your files here or click to select files";
  }

  const onDragOver = (event) => {
    event.preventDefault();
    dispatch({ type: "dragover", event });
  };

  const onDragLeave = () => {
    dispatch({ type: "dragleave" });
  };

  const onDragDrop = (event) => {
    event.preventDefault();
    dispatch({ type: "drop", event });
  };

  useEffect(() => {
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
        dispatch({ type: "add", files: event.target.files });
      });
      input.click();
    }
  };

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
