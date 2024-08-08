import { FunctionComponent } from "preact";
import { html } from "htm/preact";
import { StateMode, useState } from "../state";
import { formatBytes } from "../utils";

const FilesSummary: FunctionComponent = () => {
  const { state, dispatch } = useState();

  let action = null;
  if (state.mode !== StateMode.Complete) {
    action = html`
      <a
        href="#"
        class="text-red-200 dark:text-red-800 hover:text-red-500"
        onclick=${(event: MouseEvent) => {
          event.preventDefault();
          dispatch({ type: "removeAll" });
        }}
        >Remove all files</a
      >
    `;
  }

  return html`
    <div class="flex flex-row justify-between p-4">
      <div class="font-semibold select-none">
        ${state.mode === StateMode.Complete && "Uploaded "}
        ${formatBytes(state.totalSize)} ${" bytes over "}
        ${state.files.length.toString()}
        ${state.files.length === 1 ? " file" : " files"}
      </div>
      <div>${action}</div>
    </div>
  `;
};

export default FilesSummary;
