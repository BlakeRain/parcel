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
      "You have not uploaded any files",
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
      "test-file.txt",
    );

    cy.get(".dropdown-button").click();
    cy.get("a[title='Delete upload']").click();

    cy.get("#upload-stats-usage").should("contain", "69 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt",
    );
  });

  it("Confirmation deletes the upload", () => {
    cy.visit("/");

    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "0 B");
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files",
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
      "test-file.txt",
    );

    cy.get(".dropdown-button").click();
    cy.get("a[title='Delete upload']").click();

    cy.get("#upload-stats-usage").should("contain", "0 B");
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files",
    );
  });
});
