import users from "../fixtures/users.json";

describe("File List", () => {
  beforeEach(() => {
    cy.initialUsers();
    cy.login(users.user);
  });

  it("Presents confirmation of deletion", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    });

    cy.get("#upload-list-refresh").click();

    cy.on("window:confirm", (confirm) => {
      expect(confirm).to.equal("Are you sure you want to delete this upload?");
      return false;
    });

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(".dropdown-button").click();
    cy.get("a[title='Delete upload']").click();

    cy.get("#upload-stats-usage").should("contain", "69 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );
  });

  it("Confirmation deletes the upload", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    });

    cy.get("#upload-list-refresh").click();

    cy.on("window:confirm", (confirm) => {
      expect(confirm).to.equal("Are you sure you want to delete this upload?");
      return true;
    });

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(".dropdown-button").click();
    cy.get("a[title='Delete upload']").click();

    cy.get("#upload-stats-usage").should("contain", "0 B");
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files"
    );
  });

  it("Can make upload public", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    });

    cy.get("#upload-list-refresh").click();

    function handleConfirm(confirm) {
      expect(confirm).to.equal(
        "Are you sure you want to allow public access to this upload?"
      );
      return true;
    }

    cy.on("window:confirm", handleConfirm);

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");

    cy.get(".dropdown-button").click();
    cy.get("a[hx-post$='/public']").should("contain", "Make public").click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "Yes");

    cy.off("window:confirm", handleConfirm);

    cy.get(".dropdown-button").click();
    cy.get("a[hx-post$='/public']").should("contain", "Make private").click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");
  });

  it("Share public link", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    function handleConfirm(confirm) {
      expect(confirm).to.equal(
        "Are you sure you want to allow public access to this upload?"
      );
      return true;
    }

    cy.on("window:confirm", handleConfirm);

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-post$='/public']").should("contain", "Make public").click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "Yes");

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/share']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content > h1").should("contain", "Share Upload");

    cy.get("@upload").then((upload) => {
      cy.get(".modal > .content pre").should(
        "contain",
        `http://localhost:3000/uploads/${upload.slug}`
      );
    });
  });

  it("Share non-public upload", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/share']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content > h1").should("contain", "Share Upload");
    cy.get(".modal > .content p").should("contain", "This upload is private");

    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Make Public")
      .click();

    cy.get(".modal > .content p").should(
      "not.contain",
      "This upload is private"
    );
    cy.get(".modal > .content > h1").should("contain", "Share Upload");

    cy.get("@upload").then((upload) => {
      cy.get(".modal > .content pre").should(
        "contain",
        `http://localhost:3000/uploads/${upload.slug}`
      );
    });
  });

  it("Can change filename", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='filename']")
      .should("have.value", "test-file.txt")
      .clear()
      .type("new-file.txt");
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "new-file.txt"
    );
  });

  it("Set public via edit modal", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-post$='/public']").should("contain", "Make public");
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='public'")
      .should("not.be.checked")
      .check();
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "Yes");

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-post$='/public']").should("contain", "Make private");
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='public'")
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(7)"
    ).should("contain", "No");
  });

  it("Manage download limit", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "∞");

    // Set a download limit of 10

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='limit']").should("be.disabled");
    cy.get(".modal > .content input[name='limit_check']")
      .should("not.be.checked")
      .check();
    cy.get(".modal > .content input[name='limit']")
      .should("not.be.disabled")
      .clear()
      .type("10");
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "10 / 10");

    // Download the file once, which should reduce the download limit

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[href$='/download']").click();

    cy.get("#upload-list-refresh").click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "9 / 10");

    // Reset the download limit via the dropdown menu

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("#uploads-table .dropdown-list a[hx-post$='/reset']")
      .should("contain", "Reset remaining")
      .should("have.attr", "title", "Reset remaining downloads to 10")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "10 / 10");

    // Download the file once more, which should reduce the download limit again

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[href$='/download']").click();

    cy.get("#upload-list-refresh").click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "9 / 10");

    // Reset the download limit via the reset button next to the download limit

    cy.get(
      "#uploads-table .uploads-table-row:nth-child(2) > :nth-child(5) a[hx-post$='/reset']"
    )
      .should("have.attr", "title", "Reset remaining downloads to 10")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "10 / 10");

    // Change the download limit to 20

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='limit_check']").should("be.checked");
    cy.get(".modal > .content input[name='limit']")
      .should("be.enabled")
      .clear()
      .type("20");
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "20 / 20");

    // Remove the download limit

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='limit_check']")
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content input[name='limit']").should("be.disabled");
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(5)"
    ).should("contain", "∞");
  });

  it("Manage download expiry", () => {
    cy.visit("/");

    cy.upload({
      filename: "test-file.txt",
      owner: users.user.username,
    }).as("upload");

    cy.get("#upload-list-refresh").click();

    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(6)"
    ).should("contain", "Never");

    // Set a download expiry of 3 days

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    const today = new Date();
    const in7Days = new Date();
    in7Days.setDate(today.getDate() + 3);

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='expiry_date']").should("be.disabled");
    cy.get(".modal > .content input[name='expiry_check']")
      .should("not.be.checked")
      .check();
    cy.get(".modal > .content input[name='expiry_date']")
      .should("not.be.disabled")
      .invoke("val", in7Days.toISOString().split("T")[0]);
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(6)"
    ).should("contain", "in 3 days");

    // Remove the download expiry

    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='expiry_check']")
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content input[name='expiry_date']").should("be.disabled");
    cy.get(".modal > .content .buttons > button:first")
      .should("contain", "Save changes")
      .click();

    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) > :nth-child(6)"
    ).should("contain", "Never");
  });
});
