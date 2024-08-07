import { html } from "../../../shared.js";
import { useState } from "../state.js";

const File = (props) => {
  let actions;
  if (props.state.complete) {
    actions = html`<span class="icon-check text-green-600"></span>`;
  } else if (props.state.error) {
    actions = html`<span class="icon-x text-red-600"></span>`;
  } else if (props.state.upload) {
    actions = html`<span
      class="icon-upload text-neutral-400 dark:text-neutral-600"
    ></span>`;
  } else {
    actions = html`
      <a
        href="#"
        class="text-neutral-400 dark:text-neutral-600 hover:text-red-500"
        onclick=${(event) => {
          event.preventDefault();
          props.dispatch({ type: "remove", index });
        }}
      >
        <span class="icon-x"></span>
      </a>
    `;
  }

  return html`
    <div class="${props.file.icon}"></div>
    <div class="truncate select-none">${props.file.name}</div>
    <div class="text-right">${actions}</div>
  `;
};

const FilesList = () => {
  const { state, dispatch } = useState();

  return html`
    <div class="grid grid-cols-[max-content_1fr_max-content] gap-2">
      ${state.files.map(
        (file, index) => html`
          <${File}
            file=${file}
            index=${index}
            state=${state}
            dispatch=${dispatch}
          />
        `,
      )}
    </div>
  `;
};

export default FilesList;
