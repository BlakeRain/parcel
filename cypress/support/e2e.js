Cypress.Commands.add("resetDatabase", () => {
  cy.task("resetDatabase", null, { log: true });
});

Cypress.Commands.add("initialUsers", () => {
  cy.task("initialUsers", null, { log: true });
});

Cypress.Commands.add("login", ({ username, password }) => {
  Cypress.log({
    name: "login",
    message: `${username} | ${password}`,
  });

  cy.request("/user/signin")
    .its("body")
    .then((body) => {
      expect(body).to.include("<form");
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
