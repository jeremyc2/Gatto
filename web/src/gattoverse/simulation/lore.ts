export const catStories: Record<
  "watercolor" | "paper" | "velvet",
  { name: string; material: string; stories: string[] }
> = {
  watercolor: {
    name: "Miso",
    material: "A little out of the picture",
    stories: [
      "At home, the rain made all the colors run. Here, it just makes things wet. I think I miss the way my world could never quite stay inside the lines.",
      "I was painting the kitchen window. Then the window was real, and I was falling through it. Have you seen a house with a blue sun? That's where I live.",
      "My job was to paint the morning. Someone must be wondering why the sky hasn't changed. I should really find my way back.",
    ],
  },
  paper: {
    name: "Fold",
    material: "From somewhere with softer edges",
    stories: [
      "I used to fold the letters at the post office. Every letter knew where it was going. Funny how I never thought to write my own address down.",
      "These buildings don't have creases. The moon isn't cut out of anything. I keep looking for the edge of the page, but the city just keeps going.",
      "If you find a little paper door, please don't let it get wet. I think it might be mine. I think it might be the way home.",
    ],
  },
  velvet: {
    name: "Bramble",
    material: "A very big cat in a very strange city",
    stories: [
      "I worked the night shift at a bakery. One minute I was asleep on a warm loaf. The next, I was here. Nothing smells right. Even the bread is different.",
      "Everyone says all black cats look alike. But your shadow is sharp, and mine is soft. We aren't even from the same kind of dark.",
      "There's someone waiting for me to open the shop. I try not to think about that too much. Sit with me a minute, will you? It's easier when there's two of us.",
    ],
  },
};
