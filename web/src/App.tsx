import "./document.css";
import "./design-system/document.stylex";

if (import.meta.env.DEV && !import.meta.env.SSR) {
  void import("virtual:stylex:css-only");
}
import { Story } from "./Story";
import { SiteHeader } from "./components/SiteHeader";
import { Features } from "./components/Features";
import { DownloadSection } from "./components/DownloadSection";
import { PortalEnding } from "./components/PortalCat";
import { lazy } from "solid-js";
import { isGattoverse } from "./route";

const Gattoverse = lazy(() => import("./gattoverse/Gattoverse"));
export default function App() {
  if (isGattoverse()) return <Gattoverse />;
  return (
    <>
      <SiteHeader />
      <main>
        <Story />
        <Features />
        <DownloadSection />
      </main>
      <PortalEnding onActivate={() => window.location.assign("/gattoverse")} />
    </>
  );
}
