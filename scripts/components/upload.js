import { html } from "../shared.js";
import register from "preact-custom-element";

const UploadForm = (props) => {
  return html`
    <h1 class="heading">New Upload</h1>
    <form class="form">
      <input type="hidden" name="csrf_token" value=${props.csrf_token} />
      <button type="button" class="button">
        <span class="" />
      </button>
    </form>
  `;
};

register(UploadForm, "parcel-upload-form", ["csrf_token"]);
