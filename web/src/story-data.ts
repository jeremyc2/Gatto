import bringImage from "../assets/images/story-bring.webp";
import uploadImage from "../assets/images/story-upload.webp";
import copyImage from "../assets/images/story-copy.webp";
export const chapters = [
  {
    index: 0,
    id: "bring",
    label: "Find",
    image: bringImage,
    alt: "Three cats explore a sunlit courtyard: a watercolor kitten finds a pansy picture, a tall folded-paper cat approaches it, and a softly lit round cat investigates a shallow GitHub tray.",
    status: "Find: bring an image or video.",
  },
  {
    index: 1,
    id: "upload",
    label: "Send",
    image: uploadImage,
    alt: "The paper cat carries the pansy picture gently in its mouth toward the GitHub tray. The watercolor kitten leads along the foreground path while the round cat watches.",
    status: "Send: upload it to GitHub.",
  },
  {
    index: 2,
    id: "copy",
    label: "Share",
    image: copyImage,
    alt: "The pansy picture lies flat inside the GitHub tray, with the three pixel-art cats gathered around it.",
    status: "Share: copy the Markdown.",
  },
] as const;
