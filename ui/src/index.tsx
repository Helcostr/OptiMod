/* @refresh reload */
import { render } from "solid-js/web";
import {
  createRouter,
  createRootRoute,
  createRoute,
  RouterProvider,
} from "@tanstack/solid-router";
import Home from "./pages/Home.tsx";
import Messages from "./pages/Messages.tsx";
import "./index.css";

const rootRoute = createRootRoute();

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: Home,
});

const messagesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/messages",
  component: Messages,
});

const routeTree = rootRoute.addChildren([indexRoute, messagesRoute]);

const router = createRouter({
  routeTree,
});

declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof router;
  }
}

render(
  () => <RouterProvider router={router} />,
  document.getElementById("root")!
);