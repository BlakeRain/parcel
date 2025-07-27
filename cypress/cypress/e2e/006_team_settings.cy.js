import users from "../fixtures/users.json";

describe("Teams", () => {
  beforeEach(() => {
    cy.initialUsers();
    cy.initialTeams();
    cy.login(users.user);
  });

  it("Should see config button", () => {
    cy.visit("/");

    cy.get("#team-tab-list > a:nth-child(1)").should("contain", "Your Uploads");
    cy.get("#team-tab-list > a:nth-child(2)")
      .should("contain", "Team A")
      .click();
  });

  it("Can change team name", () => {
    cy.visit("/teams/team-a");

    // The team settings button should be visible. Click the button, and then check the modal
    // is open and that the team name is correct, then change it.
    cy.get("#team-settings-button").should("be.visible").click();
    cy.get(".modal > .content input[name='name']")
      .should("have.value", "Team A")
      .clear()
      .type("New Team Name");
    cy.get(".modal > .content button[type='submit']").click();

    // Check if the team name has been updated
    cy.url().should("eq", Cypress.config().baseUrl + "/teams/team-a");
    cy.get("#team-tab-list > a:nth-child(1)").should("contain", "Your Uploads");
    cy.get("#team-tab-list > a:nth-child(2)").should(
      "contain",
      "New Team Name"
    );
  });

  it("Can change team slug", () => {
    cy.visit("/teams/team-a");

    // Click the team settings button to open the modal, and modify the team slug.
    cy.get("#team-settings-button").should("be.visible").click();
    cy.get(".modal > .content input[name='slug']")
      .should("have.value", "team-a")
      .clear()
      .type("my-first-team")
      .blur();
    cy.get(".modal > .content button[type='submit']").click();

    // The URL should now reflect the new slug.
    cy.url().should("eq", Cypress.config().baseUrl + "/teams/my-first-team");
  });

  it("Can remove config permission", () => {
    cy.visit("/teams/team-a");

    // Click the team settings button to open the modal, and remove the config permission.
    cy.get("#team-settings-button").should("be.visible").click();

    cy.get(".modal > .content")
      .contains("label", "User")
      .next()
      .next()
      .next()
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content button[type='submit']").click();

    // The user should no longer have the config permission.
    cy.get("#team-settings-button").should("not.exist");
  });
  it("Can remove delete permission", () => {
    // Upload a file to the team.
    cy.upload({
      filename: "test-file.txt",
      owner: "team-a",
    });

    // Visit the team page.
    cy.visit("/teams/team-a");

    cy.on("window:confirm", (confirm) => {
      expect(confirm).to.equal("Are you sure you want to delete this upload?");
      return false;
    });

    // See if the delete button is visible.
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );
    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[title='Delete upload']")
      .should("be.visible")
      .and("contain", "Delete upload")
      .click();

    // Selecting a checkbox should allow the delete button to be clicked.
    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) input[type='checkbox']"
    )
      .should("be.visible")
      .check();
    cy.get("#delete_selected").should("be.visible").and("be.enabled");

    // Click the team settings button to open the modal, and remove the delete permission.
    cy.get("#team-settings-button").should("be.visible").click();
    cy.get(".modal > .content")
      .contains("label", "User")
      .next()
      .next()
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content button[type='submit']").click();

    // See if the delete button is disabled but visible.
    cy.get("#uploads-table .dropdown-button").click();
    cy.get("#uploads-table > .uploads-table-row:nth-child(2) .dropdown-menu")
      .contains("div", "Delete upload")
      .should("be.visible");

    // Selecting a checkbox should not allow the delete button to be clicked.
    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) input[type='checkbox']"
    )
      .should("be.visible")
      .check();
    cy.get("#delete_selected").should("be.visible").and("be.disabled");
  });

  it("Can remove edit permission", () => {
    // Upload a file to the team.
    cy.upload({
      filename: "test-file.txt",
      owner: "team-a",
    });

    // Visit the team page.
    cy.visit("/teams/team-a");

    // See if the edit button is visible.
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").should(
      "contain",
      "test-file.txt"
    );
    cy.get("#uploads-table .dropdown-button").click();
    cy.get("a[hx-get$='/edit']")
      .should("be.visible")
      .and("contain", "Edit upload")
      .click();

    // This should open the edit modal.
    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='filename']").should(
      "have.value",
      "test-file.txt"
    );
    cy.get(".modal > .content").contains("button", "Cancel").click();

    // Click the team settings button to open the modal, and remove the edit permission.
    cy.get("#team-settings-button").should("be.visible").click();
    cy.get(".modal > .content")
      .contains("label", "User")
      .next()
      .should("be.checked")
      .uncheck();
    cy.get(".modal > .content button[type='submit']").click();

    // See if the edit button is disabled but visible.
    cy.get("#uploads-table .dropdown-button").click();
    cy.get("#uploads-table > .uploads-table-row:nth-child(2) .dropdown-menu")
      .contains("div", "Edit upload")
      .should("be.visible");

    // See if the delete button is disabled but visible.
    cy.get("#uploads-table > .uploads-table-row:nth-child(2) .dropdown-menu")
      .contains("div", "Delete upload")
      .should("be.visible");

    // Selecting a checkbox should not allow the delete button to be clicked.
    cy.get(
      "#uploads-table > .uploads-table-row:nth-child(2) input[type='checkbox']"
    )
      .should("be.visible")
      .check();
    cy.get("#delete_selected").should("be.visible").and("be.disabled");
  });
});
