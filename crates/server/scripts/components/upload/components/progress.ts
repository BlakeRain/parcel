import { FunctionComponent } from "preact";
import { html } from "htm/preact";
import { useState } from "../state";
import { formatBytes } from "../utils";

const UploadProgress: FunctionComponent = () => {
  const { state } = useState();

  const uploaded = formatBytes(state.uploadedBytes, 2);
  const total = formatBytes(state.totalSize, 2);
  const info = `${uploaded} of ${total}`;

  return html`
    <div class="grid grid-cols-[1fr_max-content] items-center gap-4 p-4">
      <div
        class="rounded-full w-40 md:w-80 dark:bg-gray-700 border border-blue-600 dark:border-0"
      >
        <div
          class="quickly bg-blue-600 text-xs font-medium text-blue-100 text-center p-0.5 leading-none rounded-full whitespace-nowrap"
          style="width: ${state.uploadProgress}%"
        >
          ${state.uploadProgress}%
        </div>
      </div>
      <div class="select-none">${info}</div>
    </div>
  `;
};

export default UploadProgress;
