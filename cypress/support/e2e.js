Cypress.Commands.add("resetDatabase", () => {
  cy.task("resetDatabase");
});

Cypress.Commands.add("initialUsers", () => {
  cy.task("initialUsers");
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
      });
    });
});
