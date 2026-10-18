import Phaser from "phaser";

type Paint = (context: CanvasRenderingContext2D) => void;

function texture(scene: Phaser.Scene, key: string, width: number, height: number, paint: Paint) {
  if (scene.textures.exists(key)) return;
  const canvas = scene.textures.createCanvas(key, width, height);
  if (!canvas) throw new Error(`Could not create ${key}.`);
  paint(canvas.context);
  canvas.refresh();
}

function ellipse(
  c: CanvasRenderingContext2D,
  x: number,
  y: number,
  rx: number,
  ry: number,
  color: string,
) {
  c.fillStyle = color;
  c.beginPath();
  c.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
  c.fill();
}

function path(c: CanvasRenderingContext2D, points: number[], color: string) {
  c.fillStyle = color;
  c.beginPath();
  for (let i = 0; i < points.length; i += 2) {
    const x = points[i] ?? 0;
    const y = points[i + 1] ?? 0;
    if (i === 0) c.moveTo(x, y);
    else c.lineTo(x, y);
  }
  c.closePath();
  c.fill();
}

function line(c: CanvasRenderingContext2D, points: number[], color: string, width = 2) {
  c.strokeStyle = color;
  c.lineWidth = width;
  c.lineCap = "round";
  c.beginPath();
  for (let i = 0; i < points.length; i += 2) {
    const x = points[i] ?? 0;
    const y = points[i + 1] ?? 0;
    if (i === 0) c.moveTo(x, y);
    else c.lineTo(x, y);
  }
  c.stroke();
}

/** Small painted textures; Phaser owns their batching, transforms and animation. */
export function createTextures(scene: Phaser.Scene) {
  texture(scene, "pack", 50, 65, (c) => {
    c.fillStyle = "#706879";
    c.fillRect(6, 8, 34, 43);
    c.fillStyle = "#b6b5bf";
    c.fillRect(4, 8, 11, 37);
    c.fillRect(32, 8, 11, 37);
    c.fillStyle = "#393c53";
    c.fillRect(5, 40, 10, 12);
    c.fillRect(32, 40, 10, 12);
    ellipse(c, 24, 25, 8, 10, "#a8f0ce");
    line(c, [18, 13, 29, 13], "#ffdba0", 3);
  });
  texture(scene, "flame", 40, 85, (c) => {
    path(c, [3, 0, 34, 0, 27, 28, 21, 83, 13, 45, 8, 56], "#ff925b");
    path(c, [11, 0, 28, 0, 20, 57], "#ffe7a6");
  });
  texture(scene, "key", 44, 44, (c) => {
    c.shadowColor = "#e2be71";
    c.shadowBlur = 10;
    c.strokeStyle = "#f6d99a";
    c.lineWidth = 4;
    c.beginPath();
    c.arc(14, 13, 8, 0, Math.PI * 2);
    c.stroke();
    line(c, [20, 20, 35, 35, 40, 30, 35, 25], "#f6d99a", 4);
  });
  texture(scene, "food", 66, 48, (c) => {
    ellipse(c, 33, 33, 30, 10, "#131726");
    path(c, [5, 15, 61, 15, 54, 34, 14, 34], "#9b7c88");
    ellipse(c, 33, 15, 28, 10, "#ddc5bb");
    ellipse(c, 33, 14, 23, 7, "#53434e");
    for (let i = 0; i < 19; i++)
      ellipse(c, 16 + ((i * 11) % 34), 10 + ((i * 7) % 9), 3, 2, i % 2 ? "#c39668" : "#a37756");
    path(c, [30, 25, 34, 25, 36, 28, 31, 30, 28, 28], "#f3dfc4");
  });
  for (const open of [false, true]) {
    texture(scene, open ? "case-open" : "case", 88, 64, (c) => {
      ellipse(c, 44, 51, 41, 11, "#131726");
      c.fillStyle = "#655064";
      c.fillRect(7, 20, 73, 30);
      path(c, [7, 20, 20, 11, 85, 13, 80, 25], "#947485");
      path(c, [80, 25, 85, 13, 85, 41, 80, 50], "#46384e");
      c.strokeStyle = "#b19b94";
      c.lineWidth = 2;
      c.strokeRect(7, 22, 72, 27);
      c.strokeRect(35, 12, 17, 8);
      for (const x of [22, 62]) {
        c.fillStyle = "#cfb48a";
        c.fillRect(x, 20, 4, 29);
      }
      c.fillStyle = "#e3c48c";
      c.fillRect(40, 29, 8, 7);
      if (open) {
        path(c, [6, 20, 9, 1, 78, 3, 79, 23], "#a18a8a");
        path(c, [12, 23, 73, 25, 72, 40, 15, 39], "#383047");
        path(c, [21, 28, 43, 25, 50, 37, 23, 38], "#b0b7a1");
        path(c, [46, 29, 64, 26, 71, 37, 49, 38], "#d5a092");
      }
    });
  }
  const clothColors = ["#bd8796", "#d1b484", "#93a996", "#8495b3", "#b2a0c4", "#d4c5ac"];
  clothColors.forEach((color, i) => {
    texture(scene, `cloth-${i}`, 66, 88, (c) => {
      if (i % 3 === 0)
        path(
          c,
          [
            15, 5, 25, 0, 40, 0, 51, 5, 65, 19, 53, 32, 45, 25, 48, 67, 17, 67, 19, 25, 12, 32, 0,
            19,
          ],
          color,
        );
      else if (i % 3 === 1) path(c, [14, 0, 53, 0, 58, 77, 40, 80, 33, 26, 25, 80, 6, 77], color);
      else path(c, [23, 0, 29, 7, 36, 7, 42, 0, 47, 22, 61, 77, 4, 77, 18, 22], color);
      line(c, [24, 17, 19, 55], "#ffffff35", 2);
      line(c, [43, 15, 47, 58], "#00000024", 2);
      c.fillStyle = "#514350";
      c.fillRect(18, 0, 4, 7);
      c.fillRect(43, 0, 4, 7);
      if (i === 3)
        for (let y = 20; y < 65; y += 9) {
          c.fillStyle = "#d4c5ac88";
          c.fillRect(21, y, 23, 3);
        }
    });
  });
  texture(scene, "sky", 1200, 720, (c) => {
    const sky = c.createLinearGradient(0, 0, 0, 720);
    sky.addColorStop(0, "#101322");
    sky.addColorStop(0.65, "#2b2942");
    sky.addColorStop(1, "#5b4153");
    c.fillStyle = sky;
    c.fillRect(0, 0, 1200, 720);
    for (let i = 0; i < 130; i++) {
      c.globalAlpha = 0.15 + (i % 5) * 0.1;
      ellipse(
        c,
        (i * 173) % 1200,
        (i * 67) % 410,
        i % 7 === 0 ? 1.4 : 0.7,
        i % 7 === 0 ? 1.4 : 0.7,
        "#f3ddba",
      );
    }
    c.globalAlpha = 1;
    const glow = c.createRadialGradient(985, 105, 10, 985, 105, 150);
    glow.addColorStop(0, "#f3ddba20");
    glow.addColorStop(1, "#f3ddba00");
    c.fillStyle = glow;
    c.fillRect(820, 0, 340, 280);
    ellipse(c, 985, 105, 37, 37, "#f3ddba");
    ellipse(c, 975, 93, 9, 9, "#e2c7a3");
    ellipse(c, 1000, 115, 12, 12, "#e7ceac");
  });
  for (let layer = 0; layer < 3; layer++) {
    texture(scene, `city-${layer}`, 1600, 720, (c) => {
      for (let i = 0; i < 25; i++) {
        const x = i * 71 - 8;
        const top = 250 + ((i * 97 + layer * 83) % 190) + layer * 60;
        const width = 58 + ((i * 13) % 35);
        c.fillStyle = ["#222438", "#302b42", "#393044"][layer] ?? "#302b42";
        c.fillRect(x, top, width, 720 - top);
        c.fillStyle = ["#353147", "#4d3b50", "#5e4757"][layer] ?? "#353147";
        c.fillRect(x - 2, top, width + 4, 4);
        if (i % 3 === 0) {
          c.fillRect(x + 9, top - 18, 13, 18);
          c.fillRect(x + 6, top - 21, 19, 4);
        }
        if (i % 5 === 0)
          line(
            c,
            [x + 36, top, x + 36, top - 39, x + 22, top - 30, x + 50, top - 32],
            "#514456",
            2,
          );
        for (let row = 0; row < 8; row++) {
          for (let column = 0; column < 3; column++) {
            const wx = x + 8 + column * 19;
            const wy = top + 16 + row * 32;
            const lit = (i * 3 + row * 7 + column * 11 + layer) % 6 < 2;
            c.fillStyle = lit
              ? (["#d2ac87", "#e1c599", "#b6a28c"][(i + row) % 3] ?? "#d2ac87")
              : "#171c2c";
            c.globalAlpha = lit ? 0.35 + layer * 0.2 : 0.7;
            c.fillRect(wx, wy, 11, 18);
            c.globalAlpha = 1;
            c.fillStyle = "#675061";
            c.fillRect(wx - 1, wy + 18, 13, 2);
            if (lit) {
              c.fillStyle = "#4e3c4b";
              c.fillRect(wx + 5, wy, 1, 18);
              // Small domestic silhouettes connect the city to the Rust game.
              if ((row + column) % 3 === 0) {
                ellipse(c, wx + 7, wy + 9, 2, 2, "#4e3c4b");
                c.fillRect(wx + 5, wy + 11, 4, 7);
              }
            }
          }
        }
      }
    });
  }
}
