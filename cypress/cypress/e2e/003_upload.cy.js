import users from "../fixtures/users.json";

describe("Uploading Files", () => {
  beforeEach(() => {
    cy.initialUsers();
    cy.login(users.user);
  });

  it("Press upload, drop file", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.contains("button", "Upload").click();

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.disabled");

    cy.get(".modal > .content")
      .contains("Drop files")
      .selectFile("cypress/uploads/test-file.txt", {
        action: "drag-drop",
      });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("contain", "69.00");
    cy.get(".modal > .content").should("contain", "test-file.txt");
    cy.contains("button", "Upload file").should("be.enabled");
  });

  it("Drop file body, remove by summary", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content").should("contain", "69.00");
    cy.get(".modal > .content").should("contain", "test-file.txt");
    cy.contains("button", "Upload file").should("be.enabled");

    cy.contains("a", "Remove all files").click();
    cy.contains("a", "Remove all files").should("not.exist");
    cy.get(".modal > .content").should("not.contain", "test-file.txt");
    cy.contains("button", "Upload file").should("not.be.enabled");
  });

  it("Drop file body, remove by cross", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content").should("contain", "69.00");
    cy.get(".modal > .content").should("contain", "test-file.txt");
    cy.contains("button", "Upload file").should("be.enabled");

    cy.get(".modal > .content a > span.icon-x").click();
    cy.contains("a", "Remove all files").should("not.exist");
    cy.get(".modal > .content").should("not.contain", "test-file.txt");
    cy.contains("button", "Upload file").should("not.be.enabled");
  });

  it("Drop file body, upload", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.enabled").click();

    cy.get(".modal > .content").should("contain", "Upload complete");
    cy.contains("button", "Finish").should("be.enabled").click();
    cy.get(".modal > .content").should("not.exist");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");

    // The rown is a .uploads-table-row but it is actually the _second_ child after the header.
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file.txt");
      cy.get("div:nth-child(3)").should("contain", "69 B");
      cy.get("div:nth-child(4)").should("contain", "0");
      cy.get("div:nth-child(5)").should("contain", "∞");
      cy.get("div:nth-child(7)").should("contain", "No");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0]
      );
    });
  });

  it("Drop file body, add file, remove all", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content").should("contain", "69.00");
    cy.get(".modal > .content").should("contain", "1 file");
    cy.get(".modal > .content").should("contain", "test-file.txt");
    cy.contains("button", "Upload file").should("be.enabled");

    cy.get(".modal > .content")
      .contains("Drop files")
      .selectFile("cypress/uploads/test-file-2.txt", {
        action: "drag-drop",
      });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("contain", "99.00");
    cy.get(".modal > .content").should("contain", "2 files");
    cy.get(".modal > .content").should("contain", "test-file-2.txt");

    cy.contains("a", "Remove all files").click();
    cy.contains("a", "Remove all files").should("not.exist");
    cy.get(".modal > .content").should("not.contain", "test-file.txt");
    cy.get(".modal > .content").should("not.contain", "test-file-2.txt");
    cy.contains("button", "Upload file").should("not.be.enabled");
  });

  it("Drop file body, add file, remove by cross", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content").should("contain", "69.00");
    cy.get(".modal > .content").should("contain", "1 file");
    cy.get(".modal > .content").should("contain", "test-file.txt");
    cy.contains("button", "Upload file").should("be.enabled");

    cy.get(".modal > .content")
      .contains("Drop files")
      .selectFile("cypress/uploads/test-file-2.txt", {
        action: "drag-drop",
      });

    // Wait for the upad to register
    cy.wait(1000);

    cy.get(".modal > .content").should("contain", "99.00");
    cy.get(".modal > .content").should("contain", "2 files");
    cy.get(".modal > .content").should("contain", "test-file-2.txt");

    cy.get(".modal > .content div:nth-child(3) > a > span.icon-x").click();

    cy.contains("a", "Remove all files").should("be.visible");
    cy.get(".modal > .content").should("not.contain", "test-file.txt");
    cy.get(".modal > .content").should("contain", "test-file-2.txt");
    cy.contains("button", "Upload file").should("be.enabled");

    cy.get(".modal > .content div:nth-child(3) > a > span.icon-x").click();

    // cy.contains("a", "Remove all files").should("not.be.visible");
    cy.get(".modal > .content").should("not.contain", "test-file-2.txt");
    cy.contains("button", "Upload file").should("be.disabled");
  });

  it("Drop file body, upload, upload again", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");

    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    cy.wait(1000);

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.enabled").click();

    cy.get(".modal > .content").should("contain", "Upload complete");
    cy.contains("button", "Upload more").should("be.enabled").click();

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.disabled");

    cy.get("body").selectFile("cypress/uploads/test-file-2.txt", {
      action: "drag-drop",
    });

    cy.wait(1000);

    cy.contains("button", "Upload file").should("be.enabled").click();
    cy.get(".modal > .content").should("contain", "Upload complete");
    cy.contains("button", "Finish").should("be.enabled").click();
    cy.get(".modal > .content").should("not.exist");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "99 B");

    // The rown is a .uploads-table-row but it is actually the _second_ child after the header.
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file-2.txt");
      cy.get("div:nth-child(3)").should("contain", "30 B");
      cy.get("div:nth-child(4)").should("contain", "0");
      cy.get("div:nth-child(5)").should("contain", "∞");
      cy.get("div:nth-child(7)").should("contain", "No");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0]
      );
    });

    cy.get("#uploads-table > .uploads-table-row:nth-child(3)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file.txt");
      cy.get("div:nth-child(3)").should("contain", "69 B");
      cy.get("div:nth-child(4)").should("contain", "0");
      cy.get("div:nth-child(5)").should("contain", "∞");
      cy.get("div:nth-child(7)").should("contain", "No");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0]
      );
    });
  });
});
