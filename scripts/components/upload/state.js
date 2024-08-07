import { html } from "../../shared.js";
import { DragFile, FileInfo } from "./files.js";
import { createContext } from "preact";
import { useReducer, useContext } from "preact/hooks";

function createInitialState() {
  return {
    dragFiles: [],
    dragIcon: "icon-file",
    dragHint: null,
    files: [],
    totalSize: 0,
    upload: null,
    uploadedBytes: 0,
    uploadProgress: 0,
    complete: false,
  };
}

function reduceStateAction(state, action) {
  switch (action.type) {
    case "dragover": {
      const dragFiles = DragFile.fromEvent(action.event);

      let dragIcon = "icon-ban";
      let dragHint = "Upload file";

      if (dragFiles.length > 1) {
        dragIcon = "icon-files";
        dragHint = `Upload ${dragFiles.length} files`;
      } else if (dragFiles.length === 1) {
        dragIcon = dragFiles[0].icon;
        dragHint = dragFiles[0].hint;
        if (!dragHint) {
          dragHint = "Upload file";
        }
      } else {
        hitn = "No files";
      }

      return {
        ...state,
        dragFiles,
        dragIcon,
        dragHint,
      };
    }

    case "dragleave":
      return {
        ...state,
        dragFiles: [],
        dragIcon: "icon-file",
        dragHint: null,
      };

    case "drop": {
      const dropped = FileInfo.fromEvent(action.event);
      const files = [...state.files, ...dropped];

      return {
        ...state,
        dragFiles: [],
        dragIcon: "icon-file",
        dragHint: null,
        files,
        totalSize: files.map((file) => file.size).reduce((a, b) => a + b, 0),
      };
    }

    case "remove": {
      const files = state.files.filter((_, index) => index !== action.index);

      return {
        ...state,
        files,
      };
    }

    case "removeall": {
      return {
        ...state,
        files: [],
      };
    }

    case "upload": {
      return {
        ...state,
        upload: action.upload,
      };
    }

    case "progress": {
      return {
        ...state,
        uploadedBytes: action.loaded,
        uploadProgress: Math.round((action.loaded / state.totalSize) * 100),
      };
    }

    case "error": {
      return {
        ...state,
        upload: null,
        error: "There was an error uploading files",
      };
    }

    case "abort": {
      return {
        ...state,
        upload: null,
        error: "The file upload was aborted",
      };
    }

    case "complete": {
      return {
        ...state,
        upload: null,
        complete: true,
      };
    }

    default:
      console.error("Unknown action:", action);
      return state;
  }
}

export const State = createContext({
  state: createInitialState(),
  dispatch: () => {
    throw new Error("Dispatching to unpopulated state context");
  },
});

export const ProvideState = (props) => {
  const [state, dispatch] = useReducer(reduceStateAction, createInitialState());

  return html`
    <${State.Provider} value=${{ state, dispatch }}> ${props.children} <//>
  `;
};

export function useState() {
  return useContext(State);
}
