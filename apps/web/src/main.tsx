import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { AppProvider } from "./app/AppContext";
import { ErrorBoundary } from "./app/ErrorBoundary";
import { AgentSidebarProvider } from "./features/assistant/AgentSidebarContext";
import { router } from "./app/router";
import { installGlobalErrorHandlers } from "./services/clientObservability";
import "./styles/globals.css";

installGlobalErrorHandlers();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <AppProvider>
        <AgentSidebarProvider>
          <RouterProvider router={router} />
        </AgentSidebarProvider>
      </AppProvider>
    </ErrorBoundary>
  </StrictMode>,
);
