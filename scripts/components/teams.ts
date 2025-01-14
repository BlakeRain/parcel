import { createContext, FunctionComponent } from "preact";
import { useContext, useMemo, useReducer } from "preact/hooks";
import { html } from "htm/preact";
import register from "preact-custom-element";
import { loadFromScript } from "../utils";

interface Team {
  id: string;
  name: string;
}

const TeamsContext = createContext<Team[]>([]);

const WithTeams: FunctionComponent<{ selector: string }> = ({
  selector,
  children,
}) => {
  const teams = useMemo(() => loadFromScript<Team[]>(selector), [selector]);

  return html`
    <${TeamsContext.Provider} value=${teams}>${children}</${TeamsContext.Provider}>
  `;
};

type TeamPermission = "edit" | "delete";

type TeamPermissions = {
  [key in TeamPermission]: boolean;
};

interface State {
  permissions: Map<string, TeamPermissions>;
}

type StateAction =
  | { type: "add"; id: string }
  | { type: "remove"; id: string }
  | { type: "set"; id: string; permission: TeamPermission }
  | { type: "clear"; id: string; permission: TeamPermission };

function createInitialState(selector?: string): State {
  const permissions = selector
    ? loadFromScript<{ [key: string]: TeamPermissions }>(selector)
    : {};
  return { permissions: new Map(Object.entries(permissions)) };
}

function reduceState(state: State, action: StateAction): State {
  switch (action.type) {
    case "add": {
      const permissions = new Map(state.permissions);
      permissions.set(action.id, { edit: false, delete: false });
      return { permissions };
    }

    case "remove": {
      const permissions = new Map(state.permissions);
      permissions.delete(action.id);
      return { permissions };
    }

    case "set": {
      const permissions = new Map(state.permissions);
      const team = permissions.get(action.id);
      if (team) {
        team[action.permission] = true;
      }
      return { permissions };
    }

    case "clear": {
      const permissions = new Map(state.permissions);
      const team = permissions.get(action.id);
      if (team) {
        team[action.permission] = false;
        if (action.permission === "edit") {
          team.delete = false;
        }
      }
      return { permissions };
    }
  }
}

const StateContext = createContext<{
  state: State;
  dispatch: (action: StateAction) => void;
}>({
  state: createInitialState(),
  dispatch: () => {
    throw new Error("attempt to use dispatch without state context");
  },
});

const WithState: FunctionComponent<{ selector?: string }> = ({
  selector,
  children,
}) => {
  const [state, dispatch] = useReducer(
    reduceState,
    createInitialState(selector),
  );

  return html`
    <${StateContext.Provider} value=${{ state, dispatch }}> ${children} <//>
  `;
};

const HiddenField: FunctionComponent<{ name?: string }> = ({ name }) => {
  const { state } = useContext(StateContext);
  return html`
    <input
      type="hidden"
      name="${name}"
      value=${JSON.stringify(Object.fromEntries(state.permissions))}
    />
  `;
};

const TeamRow: FunctionComponent<{ team: Team }> = ({ team }) => {
  const { state, dispatch } = useContext(StateContext);
  const permissions = state.permissions.get(team.id);

  return html`
    <input
      id="team-${team.id}"
      type="checkbox"
      style="margin: 0;"
      checked=${typeof permissions !== "undefined"}
      onChange=${(event: InputEvent) => {
        const target = event.target as HTMLInputElement;
        if (target.checked) {
          dispatch({ type: "add", id: team.id });
        } else {
          dispatch({ type: "remove", id: team.id });
        }
      }}
    />
    <label
      for="team-${team.id}"
      class="${permissions ? "" : "opacity-75 italic"}"
      style="margin: 0;"
      >${team.name}</label
    >
    <input
      type="checkbox"
      style="margin: 0;"
      disabled=${!permissions}
      checked=${permissions?.edit}
      onChange=${(event: InputEvent) => {
        const target = event.target as HTMLInputElement;
        if (target.checked) {
          dispatch({ type: "set", id: team.id, permission: "edit" });
        } else {
          dispatch({ type: "clear", id: team.id, permission: "edit" });
        }
      }}
    />
    <input
      type="checkbox"
      style="margin: 0;"
      disabled=${!permissions || !permissions.edit}
      checked=${permissions?.delete}
      onChange=${(event: InputEvent) => {
        const target = event.target as HTMLInputElement;
        if (target.checked) {
          dispatch({ type: "set", id: team.id, permission: "delete" });
        } else {
          dispatch({ type: "clear", id: team.id, permission: "delete" });
        }
      }}
    />
  `;
};

const TeamRows: FunctionComponent = () => {
  const teams = useContext(TeamsContext);
  return html` ${teams.map((team) => html`<${TeamRow} team=${team} />`)} `;
};

const TeamHeadings: FunctionComponent = () => {
  return html`
    <span class="font-medium text-sm text-gray-900 dark:text-white col-start-2"
      >Team</span
    >
    <span class="font-medium text-sm text-gray-900 dark:text-white">Edit</span>
    <span class="font-medium text-sm text-gray-900 dark:text-white"
      >Delete</span
    >
  `;
};

const ParcelTeams: FunctionComponent<{
  name: string;
  class?: string;
  teams?: string;
  membership?: string;
}> = (props) => {
  return html`
    <${WithTeams} selector=${props.teams}>
      <${WithState} selector=${props.membership}>
        <${HiddenField} name=${props.name} />
        <div
          class="sm:text-sm bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg p-2.5 grid grid-cols-[max-content_1fr_repeat(2,max-content)] items-center gap-1 ${props.class}"
        >
          <${TeamHeadings} />
          <${TeamRows} />
        </div>
      <//>
    <//>
  `;
};

register(ParcelTeams, "parcel-teams", ["name", "class", "teams", "membership"]);
