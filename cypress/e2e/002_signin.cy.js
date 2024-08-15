import users from "../fixtures/users.json";

describe("Sign In", () => {
  beforeEach(() => {
    cy.resetDatabase();
    cy.initialUsers();
  });

  it("Presents interface", () => {
    cy.visit("/user/signin");
    cy.title().should("eq", "Sign In - Parcel File Sharing");
  });

  it("Disallows empty username", () => {
    cy.visit("/user/signin");
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/user/signin");
    cy.get("input[name=username]").should("have.focus");
  });

  it("Disallows empty password", () => {
    cy.visit("/user/signin");
    cy.get("input[name=username]").type(users.admin.username);
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/user/signin");
    cy.get("input[name=password]").should("have.focus");
  });

  it("Rejects unknown username", () => {
    cy.visit("/user/signin");
    cy.get("input[name=username]").type(users.admin.username + "__unknown");
    cy.get("input[name=password]").type(users.admin.password);
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/user/signin");
    cy.get("#error").should("contain", "Invalid username or password");
  });

  it("Rejects invalid password", () => {
    cy.visit("/user/signin");
    cy.get("input[name=username]").type(users.admin.username);
    cy.get("input[name=password]").type(users.admin.password + "__invalid");
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/user/signin");
    cy.get("#error").should("contain", "Invalid username or password");
  });

  it("Successful sign in (of admin)", () => {
    cy.visit("/user/signin");
    cy.get("input[name=username]").type(users.admin.username);
    cy.get("input[name=password]").type(users.admin.password);
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/admin");
  });

  it("Successful sign in (of user)", () => {
    cy.visit("/user/signin");
    cy.get("input[name=username]").type(users.user.username);
    cy.get("input[name=password]").type(users.user.password);
    cy.get("button[type=submit]").click();
    cy.url().should("eq", Cypress.config().baseUrl + "/");
  });
});
