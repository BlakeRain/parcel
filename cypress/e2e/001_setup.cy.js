import users from "../fixtures/users.json";

describe("Site Setup", () => {
  beforeEach(() => {
    cy.resetDatabase();
  });

  it("Redirects to setup if no user", () => {
    cy.visit("/");
    cy.url().should("eq", Cypress.config().baseUrl + "/admin/setup");
  });

  it("Presents interface", () => {
    cy.visit("/admin/setup");

    cy.title().should("contain", "Setup");
    cy.get("h1").should("contain", "Initial Setup");
  });

  it("Disallows empty inputs", () => {
    cy.visit("/admin/setup");
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/admin/setup");
    cy.get("input:invalid").should("have.length.greaterThan", 0);
  });

  it("Expects password confirmation", () => {
    cy.visit("/admin/setup");
    cy.get("input[name=username]").type(users.admin.username);
    cy.get("input[name=password]").type(users.admin.password);
    cy.get("input[name=confirm]").type(users.admin.password + "_modified");
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/admin/setup");
    cy.get("#password-error").should("contain", "passwords do not match");
  });

  it("Successful setup", () => {
    cy.visit("/admin/setup");

    cy.title().should("contain", "Setup");
    cy.get("h1").should("contain", "Initial Setup");

    cy.get("input[name=username]").type(users.admin.username);
    cy.get("input[name=password]").type(users.admin.password);
    cy.get("input[name=confirm]").type(users.admin.password);
    cy.get("button[type=submit]").click();

    cy.url().should("eq", Cypress.config().baseUrl + "/admin");
    cy.title().should("eq", "Administration - Parcel File Sharing");

    cy.visit("/admin/setup");
    cy.url().should("eq", Cypress.config().baseUrl + "/admin");
    cy.title().should("eq", "Administration - Parcel File Sharing");
  });
});
