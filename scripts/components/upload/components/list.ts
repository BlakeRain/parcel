import { FunctionComponent, VNode } from "preact";
import { html } from "htm/preact";
import { StateMode, useState } from "../state";
import { FileInfo } from "../files";

const File: FunctionComponent<{
  file: FileInfo;
  index: number;
}> = (props) => {
  const { state, dispatch } = useState();
  let actions: VNode;

  switch (state.mode) {
    case StateMode.Preparing:
    case StateMode.Aborted:
    case StateMode.Error:
      actions = html`
        <a
          href="#"
          class="text-neutral-400 dark:text-neutral-600 hover:text-red-500"
          onclick=${(event: MouseEvent) => {
            event.preventDefault();
            dispatch({ type: "remove", index: props.index });
          }}
        >
          <span class="icon-x"></span>
        </a>
      `;
      break;

    case StateMode.Uploading:
      actions = html`<span
        class="icon-upload text-neutral-400 dark:text-neutral-600"
      ></span>`;
      break;

    case StateMode.Complete:
      actions = html`<span class="icon-check text-green-600"></span>`;
      break;
  }

  return html`
    <div class="${props.file.icon}"></div>
    <div class="truncate select-none">${props.file.name}</div>
    <div class="text-right">${actions}</div>
  `;
};

const FilesList = () => {
  const { state } = useState();

  return html`
    <div class="grid grid-cols-[max-content_1fr_max-content] gap-2">
      ${state.files.map(
        (file, index) => html` <${File} file=${file} index=${index} /> `,
      )}
    </div>
  `;
};

export default FilesList;
