import { html } from "../../../shared.js";
import { useState } from "../state.js";
import { formatBytes } from "../utils.js";

const FilesSummary = () => {
  const { state, dispatch } = useState();

  let action = null;
  if (!state.complete) {
    action = html`
      <a
        href="#"
        class="text-red-200 dark:text-red-800 hover:text-red-500"
        onclick=${(event) => {
          event.preventDefault();
          dispatch({ type: "removeall" });
        }}
        >Remove all files</a
      >
    `;
  }

  return html`
    <div class="flex flex-row justify-between p-4">
      <div class="font-semibold select-none">
        ${state.complete && "Uploaded "} ${formatBytes(state.totalSize)}
        ${"bytes over "} ${state.files.length.toString()}
        ${state.files.length === 1 ? " file" : " files"}
      </div>
      <div>${action}</div>
    </div>
  `;
};

export default FilesSummary;
