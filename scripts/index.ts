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

function setupDropIndicator(team?: string) {
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
    if (document.querySelector("parcel-upload-form")) {
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
    if (document.querySelector("parcel-upload-form")) {
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
    if (document.querySelector("parcel-upload-form")) {
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

  document["__dropIndicatorInstalled"] = true;
}

function setupParcelChangeEvent(team?: string) {
  if (document["__parcelChangeEventInstalled"]) {
    console.log("Parcel change event already installed");
    return;
  }

  console.log("Setting up parcel change event");

  document.body.addEventListener(
    "parcelUploadChanged",
    (event: CustomEvent) => {
      const row = document.getElementById("upload-row-" + event.detail.value);
      const page = row.dataset.page;
      const order = row.dataset.order;
      const asc = row.dataset.asc;

      htmx.ajax(
        "get",
        (team ? `/teams/${team}` : "") +
          `/uploads/list/${page}?order=${order}&asc=${asc}`,
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

export function setupIndex(team?: string) {
  setupParcelChangeEvent(team);
  setupDropIndicator(team);
}
