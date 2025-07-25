import users from "../fixtures/users.json";

describe("Teams", () => {
  beforeEach(() => {
    cy.initialUsers();
    cy.login(users.admin);
  });

  it("Team administration", () => {
    // Add a new team
    cy.visit("/admin/teams");

    cy.get("button").contains("Add Team").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content h1").should("contain", "Add new team");

    cy.get(".modal > .content input[name='name']").type("First Team");
    cy.get(".modal > .content input[name='slug']").should(
      "have.value",
      "first-team",
    );

    cy.get(".modal > .content button[type='submit']").click();

    // Get the team ID from the `<parcel-clipboard>` element's `value` attribute.
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:first-child parcel-clipboard",
    )
      .invoke("attr", "value")
      .should("not.be.empty")
      .as("teamId");

    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(2)",
    ).should("contain", "First Team");

    // Enabled
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(3)",
    ).should("contain", "Yes");

    // Members
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(4)",
    ).should("contain", "0");

    // Uploads
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(5)",
    ).should("contain", "0");

    // Total
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(6)",
    ).should("contain", "0");

    // Limit
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(7)",
    ).should("contain", "1 GB");

    // Created at
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(8)",
    ).should("contain", new Date().toISOString().split("T")[0]);

    // Add user to the team
    cy.visit("/admin/users");

    cy.contains("#user-list-container table > tbody > tr", "admin").within(
      () => {
        // Currently no teams for the admin.
        cy.get("td:nth-child(8)").should("contain", "0");

        cy.get("td:nth-child(12) .dropdown-button").click();
        cy.get("a[title='Edit user']").click();
      },
    );

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content h1").should("contain", "Edit user (admin)");

    cy.get("@teamId").then((teamId) => {
      cy.get(`.modal > .content #target-${teamId}`)
        .should("not.be.checked")
        .check();

      cy.get(`.modal > .content #target-${teamId}`)
        .next()
        .next()
        .should("not.be.checked")
        .check()
        .next()
        .should("not.be.checked")
        .check();
    });

    cy.get(".modal > .content button[type='submit']").click();

    cy.contains("#user-list-container table > tbody > tr", "admin").within(
      () => {
        // Admin is now in one team.
        cy.get("td:nth-child(8)").should("contain", "1");
      },
    );

    // Go back to the teams list
    cy.visit("/admin/teams");

    // Check that the team has one member, but still no uploads.
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(4)",
    ).should("contain", "1");

    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:nth-child(5)",
    ).should("contain", "0");

    // ------------------------------------------------------------------------

    // Got to the home page and check that the team is listed in the tabs.
    cy.visit("/");

    cy.get("#team-tab-list > a:nth-child(1)").should("contain", "Your Uploads");
    cy.get("#team-tab-list > a:nth-child(2)").should("contain", "First Team");

    // The "Your Uploads" tab should be active (having the 'active' class).
    cy.get("#team-tab-list > a:nth-child(1)").should("have.class", "active");

    // There should be no uploads for the user yet.
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files",
    );

    // Upload a file to the user's uploads.
    cy.get("body").selectFile("cypress/uploads/test-file.txt", {
      action: "drag-drop",
    });

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.enabled").click();
    cy.get(".modal > .content").should("contain", "Upload complete");
    cy.contains("button", "Finish").should("be.enabled").click();
    cy.get(".modal > .content").should("not.exist");

    // Make sure the upload is listed in the user's uploads.
    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file.txt");
      cy.get("div:nth-child(3)").should("contain", "69 B");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0],
      );
    });

    // ------------------------------------------------------------------------

    // Clicking on the team tab should show navigate to the teams page.
    cy.get("#team-tab-list > a:nth-child(2)").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/teams/first-team");
    cy.get("#team-tab-list > a:nth-child(1)").should(
      "not.have.class",
      "active",
    );
    cy.get("#team-tab-list > a:nth-child(2)").should("have.class", "active");

    // There should be no uploads for the team yet.
    cy.get("#uploads-table").should(
      "contain",
      "You have not uploaded any files",
    );

    // Upload a file to the team's uploads.
    cy.get("body").selectFile("cypress/uploads/test-file-2.txt", {
      action: "drag-drop",
    });

    cy.get(".modal > .content").should("be.visible");
    cy.contains("button", "Upload file").should("be.enabled").click();
    cy.get(".modal > .content").should("contain", "Upload complete");
    cy.contains("button", "Finish").should("be.enabled").click();
    cy.get(".modal > .content").should("not.exist");

    // Make sure the upload is listed in the team's uploads.
    cy.get("#upload-stats-progress").should("contain", "< 1%");
    cy.get("#upload-stats-usage").should("contain", "30 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file-2.txt");
      cy.get("div:nth-child(3)").should("contain", "30 B");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0],
      );
    });

    // ------------------------------------------------------------------------

    // Clicking the "Your Uploads" tab should navigate back to the user's uploads (exactly `/`).
    cy.get("#team-tab-list > a:nth-child(1)").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/");
    cy.get("#team-tab-list > a:nth-child(1)").should("have.class", "active");
    cy.get("#team-tab-list > a:nth-child(2)").should(
      "not.have.class",
      "active",
    );

    // Make sure the upload is listed in the user's uploads.
    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file.txt");
      cy.get("div:nth-child(3)").should("contain", "69 B");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0],
      );
    });

    // ------------------------------------------------------------------------

    // Clicking the back button should navigate back to the team page.
    cy.go("back");
    cy.url().should("eq", Cypress.config().baseUrl + "/teams/first-team");
    cy.get("#team-tab-list > a:nth-child(1)").should(
      "not.have.class",
      "active",
    );
    cy.get("#team-tab-list > a:nth-child(2)").should("have.class", "active");

    // Make sure the upload is listed in the team's uploads.
    cy.get("#upload-stats-progress").should("contain", "< 1%");
    cy.get("#upload-stats-usage").should("contain", "30 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file-2.txt");
      cy.get("div:nth-child(3)").should("contain", "30 B");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0],
      );
    });

    // Clicking the back button again should navigate back to the user's uploads.
    cy.go("back");
    cy.url().should("eq", Cypress.config().baseUrl + "/");
    cy.get("#team-tab-list > a:nth-child(1)").should("have.class", "active");
    cy.get("#team-tab-list > a:nth-child(2)").should(
      "not.have.class",
      "active",
    );

    // ------------------------------------------------------------------------

    // Make sure the upload is listed in the user's uploads.
    cy.get("#upload-stats-progress").should("contain", "0%");
    cy.get("#upload-stats-usage").should("contain", "69 B");
    cy.get("#uploads-table > .uploads-table-row:nth-child(2)").within(() => {
      cy.get("div:nth-child(2)").should("contain", "test-file.txt");
      cy.get("div:nth-child(3)").should("contain", "69 B");
      cy.get("div:nth-child(8)").should(
        "contain",
        new Date().toISOString().split("T")[0],
      );
    });
  });

  it("Team configuration", () => {
    // Add a new team
    cy.visit("/admin/teams");

    cy.get("button").contains("Add Team").click();

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content h1").should("contain", "Add new team");

    cy.get(".modal > .content input[name='name']").type("First Team");
    cy.get(".modal > .content input[name='slug']").should(
      "have.value",
      "first-team",
    );

    cy.get(".modal > .content button[type='submit']").click();

    // Get the team ID from the `<parcel-clipboard>` element's `value` attribute.
    cy.get(
      "#teams-list-container table > tbody > tr:first-child > td:first-child parcel-clipboard",
    )
      .invoke("attr", "value")
      .should("not.be.empty")
      .as("teamId");

    // Add user to the team
    cy.visit("/admin/users");

    cy.contains("#user-list-container table > tbody > tr", "admin").within(
      () => {
        cy.get("td:nth-child(12) .dropdown-button").click();
        cy.get("a[title='Edit user']").click();
      },
    );

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content h1").should("contain", "Edit user (admin)");

    cy.get("@teamId").then((teamId) => {
      cy.get(`.modal > .content #target-${teamId}`)
        .should("not.be.checked")
        .check();

      cy.get(`.modal > .content #target-${teamId}`)
        .next()
        .next()
        .should("not.be.checked")
        .check()
        .next()
        .should("not.be.checked")
        .check();
    });

    cy.get(".modal > .content button[type='submit']").click();

    // Visit the team page and make sure the configuration option is not available yet.
    cy.visit("/teams/first-team");
    cy.get("#team-settings-button").should("not.exist");

    // Go back to the user page and edit the team permissions for the user.
    cy.visit("/admin/users");

    cy.contains("#user-list-container table > tbody > tr", "admin").within(
      () => {
        cy.get("td:nth-child(12) .dropdown-button").click();
        cy.get("a[title='Edit user']").click();
      },
    );

    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content h1").should("contain", "Edit user (admin)");

    cy.get("@teamId").then((teamId) => {
      cy.get(`.modal > .content #target-${teamId}`)
        .next()
        .next()
        .next()
        .next()
        .should("not.be.checked")
        .check();
    });

    cy.get(".modal > .content button[type='submit']").click();

    // Visit the team page and make sure the configuration option is avaialable now.
    cy.visit("/teams/first-team");
    cy.get("#team-settings-button").should("be.visible").click();

    // Check the modal is open and that the team name is correct, then change it.
    cy.get(".modal > .content").should("be.visible");
    cy.get(".modal > .content input[name='name']")
      .should("have.value", "First Team")
      .clear()
      .type("New Team Name");

    cy.get(".modal > .content button[type='submit']").click();

    cy.url().should("eq", Cypress.config().baseUrl + "/teams/first-team");
    cy.get("#team-tab-list > a:nth-child(1)").should("contain", "Your Uploads");
    cy.get("#team-tab-list > a:nth-child(2)").should(
      "contain",
      "New Team Name",
    );

    // Open the team settings modal again and change the slug.
    cy.get("#team-settings-button").should("be.visible").click();
    cy.get(".modal > .content input[name='slug']").should(
      "have.value",
      "first-team",
    );
    cy.get(".modal > .content input[name='slug']")
      .should("have.value", "first-team")
      .clear()
      .type("my-first-team");

    // Let the modal settle.
    cy.wait(1000);

    cy.get(".modal > .content button[type='submit']").click();
  });
});
