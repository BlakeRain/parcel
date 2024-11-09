import users from "../fixtures/users.json";

Cypress.Commands.add("resetDatabase", () => {
  cy.request("/debug/reset-database").then((response) => {
    expect(response.status).to.eq(200);
  });
});

Cypress.Commands.add("initialUsers", () => {
  const body = Object.keys(users).map((key) => users[key]);
  cy.request("POST", "/debug/initial-users", body).then((response) => {
    expect(response.status).to.eq(200);
  });
});

Cypress.Commands.add("login", ({ username, password }) => {
  Cypress.log({
    name: "login",
    message: `${username} | ${password}`,
  });

  cy.request("/user/signin")
    .its("body")
    .then((body) => {
      const $html = Cypress.$(body);
      const token = $html.find("input[name=token]").val();

      cy.request({
        method: "POST",
        url: "/user/signin",
        form: true,
        body: {
          token,
          username,
          password,
        },
      }).then((response) => {
        expect(response.status).to.eq(200);
      });
    });
});
