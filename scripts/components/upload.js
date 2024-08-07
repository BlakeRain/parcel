import { html } from "../shared.js";
import register from "preact-custom-element";

const UploadForm = (props) => {
  const onCancelClick = (event) => {
    event.preventDefault();
    event.target.closest("parcel-modal").closeModal();
  };

  return html`
    <h1 class="heading">New Upload</h1>
    <form class="form">
      <input type="hidden" name="csrf_token" value=${props.csrf_token} />
      <div>Form fields go here</div>
      <div class="buttons end mt-2">
        <a href="#" class="mx-2" onclick=${onCancelClick}> Cancel </a>
        <button type="button" class="button">
          <span class="icon-upload"></span>
          Upload file
        </button>
      </div>
    </form>
  `;
};

register(UploadForm, "parcel-upload-form", ["csrf_token"]);
