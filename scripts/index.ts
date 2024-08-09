class DropIndicator {
  private element: HTMLElement;
  private visible: boolean = false;
  private start: EventTarget;

  constructor() {
    this.element = document.getElementById("drop-indicator");

    this.element.addEventListener("animationend", () => {
      if (this.element.classList.contains("closing")) {
        this.element.classList.remove("closing");
        this.element.classList.add("invisible");
      } else if (this.element.classList.contains("opening")) {
        this.element.classList.remove("opening");
      }
    });
  }

  public onDragEnter(event: DragEvent) {
    this.start = event.target;
    event.preventDefault();
    event.stopPropagation();
    this.show();
  }

  public onDragLeave(event: DragEvent) {
    if (this.start === event.target) {
      event.preventDefault();
      event.stopPropagation();
      this.hide(true);
    }
  }

  show() {
    if (this.visible) {
      return;
    }

    this.element.classList.remove("invisible");
    this.element.classList.add("opening");
    this.visible = true;
  }

  hide(animated: boolean) {
    if (!this.visible) {
      return;
    }

    if (animated) {
      this.element.classList.add("closing");
    } else {
      this.element.classList.add("invisible");
    }

    this.visible = false;
  }
}

let DROP_INDICATOR: DropIndicator = null;
function getDropIndicator(): DropIndicator {
  if (!DROP_INDICATOR) {
    DROP_INDICATOR = new DropIndicator();
  }

  return DROP_INDICATOR;
}

document.body.addEventListener("parcelUploadDeleted", () => {
  htmx.trigger("#upload-stats-container", "refresh");
});

// Always prevent the default action.
document.body.addEventListener("dragover", (event: DragEvent) => {
  event.preventDefault();
});

document.body.addEventListener("dragenter", (event: DragEvent) => {
  // First we want to see if we already have a `<parcel-upload-form>` in the DOM. If we do, then we
  // don't need to do anything here, as the form handles this event.
  if (document.querySelector("parcel-upload-form")) {
    return;
  }

  getDropIndicator().onDragEnter(event);
});

document.body.addEventListener("dragleave", (event: DragEvent) => {
  // As with 'handleDragOver', we're not going to operate if we have a `<parcel-upload-form>` in the
  // DOM, as that'll handle the event.
  if (document.querySelector("parcel-upload-form")) {
    return;
  }

  getDropIndicator().onDragLeave(event);
});

document.body.addEventListener("drop", (event: DragEvent) => {
  // The user has dropped some files directly into the index page. First off, we want to see if
  // there is already a `<parcel-upload-form>` that can handle that for us.
  if (document.querySelector("parcel-upload-form")) {
    return;
  }

  // First we want to prevent the default process for the event.
  event.preventDefault();

  // Hide the drop indicator, but don't animate.
  getDropIndicator().hide(false);

  const files = [...event.dataTransfer.items]
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile());

  // There is no form, present, so we need to load one. We can do that with HTMX. We tell the upload
  // form not to bother animating in.
  htmx
    .ajax("GET", "/uploads/new?immediate=true", {
      target: "body",
      swap: "beforeend",
    })
    .then(() => {
      // Now we can get at the `<upload-form>` and tell it about our dropped files. However the
      // event receiver will not mount immediately: whilst HTMX things things are on their way to
      // being settled, we would still have to wait for the upload form to populate and connect
      // itself together.

      let count = 0;

      const interval = window.setInterval(() => {
        const receiver = document.querySelector(
          "parcel-upload-form > .event-receiver",
        );
        if (receiver) {
          window.clearInterval(interval);
          receiver.dispatchEvent(
            new CustomEvent("parcelDrop", {
              detail: { files },
            }),
          );
        } else if (count > 10) {
          console.error("Failed to find event receiver in upload form");
          window.clearInterval(interval);
        } else {
          count++;
        }
      }, 100);
    });
});
