import { html } from "htm/preact";
import { DragFile, FileInfo } from "./files";
import { createContext, FunctionComponent } from "preact";
import { useReducer, useContext } from "preact/hooks";

export type StateAction =
  | { type: "dragover"; event: DragEvent }
  | { type: "dragleave" }
  | { type: "drop"; event: DragEvent }
  | { type: "add"; files: FileList }
  | { type: "remove"; index: number }
  | { type: "removeAll" }
  | { type: "upload"; upload: XMLHttpRequest }
  | { type: "progress"; loaded: number }
  | { type: "error"; event: ProgressEvent<any> }
  | { type: "abort"; event: ProgressEvent<any> }
  | { type: "complete" }
  | { type: "reset" };

export enum StateMode {
  Preparing,
  Uploading,
  Aborted,
  Error,
  Complete,
}

export interface State {
  mode: StateMode;
  dragFiles: DragFile[];
  dragIcon: string;
  dragHint: string | null;
  files: FileInfo[];
  totalSize: number;
  upload: XMLHttpRequest | null;
  uploadedBytes: number;
  uploadProgress: number;
  error: string | null;
}

function createInitialState(): State {
  return {
    mode: StateMode.Preparing,
    dragFiles: [],
    dragIcon: "icon-file",
    dragHint: null,
    files: [],
    totalSize: 0,
    upload: null,
    uploadedBytes: 0,
    uploadProgress: 0,
    error: null,
  };
}

function reduceStateAction(state: State, action: StateAction): State {
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
        dragHint = "No files";
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

    case "add": {
      const added = FileInfo.fromList(action.files);
      const files = [...state.files, ...added];

      return {
        ...state,
        files,
        totalSize: files.map((file) => file.size).reduce((a, b) => a + b, 0),
      };
    }

    case "remove": {
      const files = state.files.filter((_, index) => index !== action.index);

      return {
        ...state,
        files,
        totalSize: files.map((file) => file.size).reduce((a, b) => a + b, 0),
      };
    }

    case "removeAll": {
      return {
        ...state,
        files: [],
        totalSize: 0,
      };
    }

    case "upload": {
      return {
        ...state,
        upload: action.upload,
        mode: StateMode.Uploading,
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
        mode: StateMode.Complete,
      };
    }

    case "reset": {
      return createInitialState();
    }

    default:
      console.error("Unknown action:", action);
      return state;
  }
}

export const State = createContext({
  state: createInitialState(),
  dispatch: (_action: StateAction) => {
    throw new Error("Dispatching to unpopulated state context");
  },
});

export const ProvideState: FunctionComponent = (props) => {
  const [state, dispatch] = useReducer(reduceStateAction, createInitialState());

  return html`
    <${State.Provider} value=${{ state, dispatch }}> ${props.children} <//>
  `;
};

export function useState() {
  return useContext(State);
}
