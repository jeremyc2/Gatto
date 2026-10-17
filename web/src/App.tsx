import "./document.css";
import "./design-system/document.stylex";

if (import.meta.env.DEV && !import.meta.env.SSR) {
  void import("virtual:stylex:css-only");
}
import { Story } from "./Story";
import { SiteHeader } from "./components/SiteHeader";
import { SiteFooter } from "./components/SiteFooter";
import { Features } from "./components/Features";
import { DownloadSection } from "./components/DownloadSection";
export default function App() {
  return (
    <>
      <SiteHeader />
      <main>
        <Story />
        <Features />
        <DownloadSection />
      </main>
      <SiteFooter />
    </>
  );
}
