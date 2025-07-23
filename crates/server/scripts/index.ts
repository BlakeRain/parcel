interface DropIndicator {
  __dropIndicator: {
    visible: boolean;
    start: EventTarget;
  };
}

function getDropIndicator(): HTMLElement & DropIndicator {
  const element = document.getElementById("drop-indicator") as HTMLElement &
    DropIndicator;
  if (!element) {
    throw new Error("Could not find drop indicator element");
  }

  if (!element.__dropIndicator) {
    element.__dropIndicator = {
      visible: false,
      start: null,
    };

    element.addEventListener("animationend", () => {
      if (element.classList.contains("closing")) {
        element.classList.remove("closing");
        element.classList.add("invisible");
      } else if (element.classList.contains("opening")) {
        element.classList.remove("opening");
      }
    });
  }

  return element;
}

function showDropIndicator(indicator: HTMLElement & DropIndicator) {
  if (indicator.__dropIndicator.visible) {
    return;
  }

  indicator.classList.remove("invisible");
  indicator.classList.add("opening");
  indicator.__dropIndicator.visible = true;
}

function hideDropIndicator(
  indicator: HTMLElement & DropIndicator,
  animated: boolean,
) {
  if (!indicator.__dropIndicator.visible) {
    return;
  }

  if (animated) {
    indicator.classList.add("closing");
  } else {
    indicator.classList.add("invisible");
  }

  indicator.__dropIndicator.visible = false;
}

// Get the team identifier from the script element with id "team-identifier".
function getTeamIdentifier(): string | null {
  const element = document.getElementById("team-identifier");
  if (!element) {
    console.error("Missing team identifier element");
    return null;
  }

  if (element.tagName !== "SCRIPT") {
    console.error("Team identifier element is not a script element");
    return null;
  }

  let identifier: string = null;
  try {
    const value = JSON.parse(element.textContent);
    if (!(value instanceof Array)) {
      console.error("Team identifier value is not an array");
      return null;
    }

    if (value.length === 1) {
      identifier = value[0];
    }
  } catch (e) {
    console.error("Failed to parse team identifier value:", e);
    return null;
  }

  return identifier;
}

// Check if there is a `<parcel-upload-form>` in the DOM.
//
// We use this to gate our drag-and-drop trigger functionality when there is no upload form
// present. If there is a form, then it will handle the drag-and-drop events for itself.
function hasUploadForm(): boolean {
  return document.querySelector("parcel-upload-form") !== null;
}

function setupDropIndicator() {
  if (document["__dropIndicatorInstalled"]) {
    console.log("Drop indicator already installed");
    return;
  }

  console.log("Setting up drop indicator");

  // Always prevent the default action.
  document.body.addEventListener("dragover", (event: DragEvent) => {
    event.preventDefault();
  });

  document.body.addEventListener("dragenter", (event: DragEvent) => {
    // First we want to see if we already have a `<parcel-upload-form>` in the DOM. If we do, then we
    // don't need to do anything here, as the form handles this event.
    if (hasUploadForm()) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const indicator = getDropIndicator();
    indicator.__dropIndicator.start = event.target;
    showDropIndicator(indicator);
  });

  document.body.addEventListener("dragleave", (event: DragEvent) => {
    // As with 'handleDragOver', we're not going to operate if we have a `<parcel-upload-form>` in the
    // DOM, as that'll handle the event.
    if (hasUploadForm()) {
      return;
    }

    const indicator = getDropIndicator();
    if (indicator.__dropIndicator.start === event.target) {
      event.preventDefault();
      event.stopPropagation();
      hideDropIndicator(indicator, true);
    }
  });

  document.body.addEventListener("drop", (event: DragEvent) => {
    // The user has dropped some files directly into the index page. First off, we want to see if
    // there is already a `<parcel-upload-form>` that can handle that for us.
    if (hasUploadForm()) {
      return;
    }

    // First we want to prevent the default process for the event.
    event.preventDefault();

    // Hide the drop indicator, but don't animate.
    const indicator = getDropIndicator();
    hideDropIndicator(indicator, false);

    const files = [...event.dataTransfer.items]
      .filter((item) => item.kind === "file")
      .map((item) => item.getAsFile());

    const team = getTeamIdentifier();

    // There is no form, present, so we need to load one. We can do that with HTMX. We tell the
    // upload form not to bother animating in.
    htmx
      .ajax(
        "get",
        "/uploads/new?immediate=true" + (team ? `&team=${team}` : ""),
        {
          target: "body",
          swap: "beforeend",
        },
      )
      .then(() => {
        // Now we can get at the `<upload-form>` and tell it about our dropped files. However the
        // event receiver will not mount immediately: whilst HTMX things are on their way to
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

  document["__dropIndicatorInstalled"] = true;
}

function setupParcelChangeEvent() {
  if (document["__parcelChangeEventInstalled"]) {
    console.log("Parcel change event already installed");
    return;
  }

  console.log("Setting up parcel change event");

  document.body.addEventListener(
    "parcelUploadChanged",
    (event: CustomEvent) => {
      const team = getTeamIdentifier();
      const row = document.getElementById("upload-row-" + event.detail.value);
      const page = row.dataset.page;
      const order = row.dataset.order;
      const asc = row.dataset.asc;

      const params = new URLSearchParams();

      if (order) {
        params.set("order", order);
      }

      if (asc) {
        params.set("asc", asc);
      }

      htmx.ajax(
        "get",
        (team ? `/teams/${team}` : "") +
          `/uploads/list/${page}?${params.toString()}`,
        {
          target: "#upload-row-" + event.detail.value,
          select: "#upload-row-" + event.detail.value,
          swap: "outerHTML",
        },
      );
    },
  );

  document["__parcelChangeEventInstalled"] = true;
}

setupParcelChangeEvent();
setupDropIndicator();
