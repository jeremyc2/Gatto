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
import { createSiteNotifications } from "./components/SiteNotifications";
export default function App() {
  const notifications = createSiteNotifications();
  return (
    <>
      <SiteHeader />
      <main>
        <Story />
        <Features />
        <DownloadSection />
      </main>
      <PortalEnding onActivate={() => notifications.show("Gattoverse mini game coming soon.")} />
      <notifications.Region />
    </>
  );
}
