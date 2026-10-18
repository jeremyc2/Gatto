import type Phaser from "phaser";

/** Code-native prop art keeps the game's scenery editable alongside its visual vocabulary. */
export function registerPropTextures(scene: Phaser.Scene) {
  const paint = (
    key: string,
    w: number,
    h: number,
    draw: (c: CanvasRenderingContext2D) => void,
  ) => {
    const texture = scene.textures.createCanvas(key, w, h);
    if (!texture) return;
    draw(texture.context);
    texture.refresh();
  };
  const ellipse = (
    c: CanvasRenderingContext2D,
    x: number,
    y: number,
    rx: number,
    ry: number,
    color: string,
  ) => {
    c.fillStyle = color;
    c.beginPath();
    c.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
    c.fill();
  };
  const polygon = (c: CanvasRenderingContext2D, pts: number[], color: string) => {
    c.fillStyle = color;
    c.beginPath();
    pts.forEach((v, i) => {
      if (i % 2) return;
      if (!i) c.moveTo(v, pts[i + 1] ?? 0);
      else c.lineTo(v, pts[i + 1] ?? 0);
    });
    c.closePath();
    c.fill();
  };
  paint("vendor", 110, 155, (c) => {
    ellipse(c, 55, 142, 50, 12, "#12182966");
    polygon(c, [15, 13, 78, 5, 98, 20, 35, 29], "#99a29c");
    polygon(c, [78, 5, 98, 20, 98, 135, 78, 145], "#294b4e");
    c.fillStyle = "#537b75";
    c.fillRect(15, 13, 63, 131);
    c.fillStyle = "#142d35";
    c.fillRect(22, 42, 43, 58);
    c.fillRect(27, 116, 37, 16);
    c.fillStyle = "#a9d8be";
    c.fillRect(25, 45, 37, 51);
    c.fillStyle = "#7e818d";
    c.fillRect(34, 53, 20, 31);
    c.fillStyle = "#d0d4d5";
    c.fillRect(30, 51, 7, 35);
    c.fillRect(51, 51, 7, 35);
    ellipse(c, 44, 64, 6, 6, "#ffe2aa");
    c.strokeStyle = "#f6e7b9";
    c.lineWidth = 3;
    c.beginPath();
    c.moveTo(32, 33);
    c.lineTo(45, 21);
    c.lineTo(57, 33);
    c.stroke();
    ellipse(c, 70, 74, 3, 4, "#ffdba0");
    c.font = "bold 13px Arial";
    c.fillStyle = "#ffe2aa";
    c.fillText("5", 49, 111);
    ellipse(c, 39, 106, 5, 5, "#ffd38c");
  });
  paint("plant", 76, 106, (c) => {
    ellipse(c, 37, 95, 34, 9, "#141a2566");
    polygon(c, [19, 68, 56, 68, 50, 96, 25, 96], "#aa7d79");
    ellipse(c, 38, 68, 20, 8, "#d2a898");
    c.strokeStyle = "#435f52";
    c.lineWidth = 4;
    c.beginPath();
    c.moveTo(38, 77);
    c.lineTo(37, 24);
    c.stroke();
    for (let i = 0; i < 12; i++)
      ellipse(
        c,
        21 + ((i * 17) % 37),
        20 + ((i * 23) % 45),
        10 + (i % 5),
        7 + (i % 8),
        ["#73957e", "#8aa18a", "#536e64"][i % 3] ?? "#73957e",
      );
  });
  paint("vent", 124, 98, (c) => {
    polygon(c, [10, 44, 57, 19, 115, 46, 68, 74], "#9996a2");
    polygon(c, [10, 44, 68, 74, 68, 94, 10, 63], "#6c7082");
    polygon(c, [68, 74, 115, 46, 115, 67, 68, 94], "#474f65");
    ellipse(c, 61, 46, 25, 14, "#4b5267");
    for (let i = 0; i < 6; i++) {
      c.strokeStyle = "#ada9b5";
      c.lineWidth = 2;
      c.beginPath();
      c.moveTo(39 + i * 7, 37);
      c.lineTo(48 + i * 6, 55);
      c.stroke();
    }
  });
  paint("chimney", 96, 126, (c) => {
    polygon(c, [18, 40, 53, 21, 78, 34, 78, 99, 43, 119, 18, 105], "#795f6f");
    polygon(c, [43, 53, 78, 34, 78, 99, 43, 119], "#594d64");
    polygon(c, [10, 34, 53, 12, 87, 30, 43, 56], "#aa929b");
    polygon(c, [25, 33, 53, 20, 73, 30, 44, 44], "#292c40");
    c.strokeStyle = "#ad8991";
    c.lineWidth = 1;
    for (let i = 0; i < 5; i++) {
      c.beginPath();
      c.moveTo(20, 53 + i * 11);
      c.lineTo(42, 65 + i * 11);
      c.stroke();
    }
  });
  paint("crate", 82, 73, (c) => {
    polygon(c, [5, 22, 43, 5, 76, 23, 76, 51, 40, 70, 5, 50], "#96776e");
    polygon(c, [40, 42, 76, 23, 76, 51, 40, 70], "#6c5a61");
    polygon(c, [5, 22, 43, 5, 76, 23, 40, 42], "#b3927c");
    c.strokeStyle = "#544957";
    c.lineWidth = 2;
    for (let i = 0; i < 3; i++) {
      c.beginPath();
      c.moveTo(6, 30 + i * 9);
      c.lineTo(39, 48 + i * 9);
      c.lineTo(75, 29 + i * 9);
      c.stroke();
    }
  });
  paint("lantern", 80, 100, (c) => {
    const glow = c.createRadialGradient(40, 58, 5, 40, 58, 40);
    glow.addColorStop(0, "#ffdba059");
    glow.addColorStop(1, "#ffdba000");
    c.fillStyle = glow;
    c.fillRect(0, 18, 80, 82);
    c.strokeStyle = "#95818e";
    c.lineWidth = 3;
    c.strokeRect(31, 38, 18, 33);
    c.beginPath();
    c.arc(40, 34, 8, Math.PI, Math.PI * 2);
    c.stroke();
    c.fillStyle = "#ffdba0";
    c.fillRect(34, 43, 12, 24);
    c.fillStyle = "#b4988b";
    c.fillRect(28, 70, 24, 4);
  });
  paint("rug", 140, 90, (c) => {
    polygon(c, [5, 32, 67, 3, 136, 51, 72, 86], "#9a737c");
    polygon(c, [15, 33, 66, 10, 125, 51, 72, 78], "#6d667a");
    polygon(c, [35, 36, 65, 21, 105, 50, 73, 66], "#a8989d");
    polygon(c, [54, 39, 65, 33, 88, 49, 74, 55], "#6e5369");
  });
  paint("puddle", 140, 76, (c) => {
    c.globalAlpha = 0.45;
    ellipse(c, 60, 42, 52, 20, "#242e48");
    ellipse(c, 97, 44, 25, 12, "#242e48");
    c.globalAlpha = 0.4;
    ellipse(c, 64, 38, 34, 3, "#9cabc2");
    ellipse(c, 91, 48, 18, 2, "#9cabc2");
  });
}
