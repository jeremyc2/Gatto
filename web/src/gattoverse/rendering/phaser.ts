import Phaser from "phaser";
import coinUrl from "../../../assets/images/gattoverse-coin.png";
import type { GameView, Point, RenderEntity } from "../simulation/contracts";
import { actorKinds, registerActorAtlases, WALK_FRAMES } from "./actor-atlas";
import { project, unproject } from "./projection";
import { registerPropTextures } from "./prop-atlas";
import { paintFlightRoof, paintRoof, type Scenery } from "./scenery";
import { createTextures } from "./textures";

export type Renderer = {
  draw(view: GameView): void;
  pick(clientX: number, clientY: number): { point: Point; entity?: number };
  sleep(hidden: boolean): void;
  destroy(): void;
};
type Drawn = { sprite: Phaser.GameObjects.Sprite; shadow: Phaser.GameObjects.Ellipse | undefined };

/** This adapter has no simulation import, physics system, input manager, or mutable game rules. */
export function createRenderer(host: HTMLElement): Promise<Renderer> {
  return new Promise((resolve, reject) => {
    let game: Phaser.Game;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    class ViewScene extends Phaser.Scene {
      private sprites = new Map<number, Drawn>();
      private scenery: Scenery = { objects: [], clothes: [] };
      private platforms = new Map<number, Phaser.GameObjects.GameObject[]>();
      private sky!: Phaser.GameObjects.Image;
      private skyline: Phaser.GameObjects.TileSprite[] = [];
      private pack!: Phaser.GameObjects.Image;
      private flame!: Phaser.GameObjects.Image;
      private ring!: Phaser.GameObjects.Graphics;
      private last: GameView | undefined;
      private sceneKey = "";
      private serial = 0;
      private snapCamera = true;
      constructor() {
        super("View");
      }
      preload() {
        this.load.image("coin", coinUrl);
      }
      create() {
        try {
          createTextures(this);
          registerPropTextures(this);
          registerActorAtlases(this);
          this.sky = this.add.image(0, 0, "sky").setOrigin(0).setDepth(-100000);
          this.skyline = [0, 1, 2].map((i) =>
            this.add
              .tileSprite(0, 0, 1200, 800, `city-${i}`)
              .setOrigin(0)
              .setDepth(-90000 + i),
          );
          this.pack = this.add.image(0, 0, "pack").setScale(0.36).setVisible(false);
          this.flame = this.add
            .image(0, 0, "flame")
            .setOrigin(0.5, 0)
            .setScale(0.3)
            .setVisible(false);
          this.ring = this.add.graphics();
          this.scale.on("resize", () => {
            this.snapCamera = true;
            if (this.last) this.draw(this.last);
          });
          resolve({
            draw: (view) => this.draw(view),
            pick: (x, y) => this.pick(x, y),
            sleep: (hidden) => {
              if (hidden) game.loop.sleep();
              else game.loop.wake();
            },
            destroy: () => game.destroy(true),
          });
        } catch (error) {
          game.destroy(true);
          reject(error);
        }
      }

      private resetScenery(view: GameView) {
        this.serial = view.feedback?.serial ?? 0;
        this.scenery.objects.forEach((object) => object.destroy());
        this.scenery = { objects: [], clothes: [] };
        this.platforms.forEach((objects) => objects.forEach((object) => object.destroy()));
        this.platforms.clear();
        this.sprites.forEach(({ sprite, shadow }) => {
          sprite.destroy();
          shadow?.destroy();
        });
        this.sprites.clear();
        if (view.mode === "roof") this.scenery = paintRoof(this, view.roof, reduced.matches);
        this.snapCamera = true;
      }

      private entityPosition(entity: RenderEntity, view: GameView) {
        return view.mode === "roof" ? project(entity) : entity;
      }

      private paintEntity(entity: RenderEntity, view: GameView) {
        const actor = actorKinds.some((kind) => kind === entity.kind);
        let key = actor
          ? `actor-${entity.kind}`
          : entity.kind === "cloth"
            ? `cloth-${entity.variant}`
            : entity.kind === "case" && entity.used
              ? "case-open"
              : entity.kind;
        if (!this.textures.exists(key)) key = "crate";
        const p = this.entityPosition(entity, view);
        let drawn = this.sprites.get(entity.id);
        if (!drawn) {
          const sprite = this.add.sprite(p.x, p.y, key).setOrigin(0.5, actor ? 0.85 : 0.84);
          drawn = {
            sprite,
            shadow: actor ? this.add.ellipse(p.x, p.y, 23, 9, 0x151b2c, 0.38) : undefined,
          };
          this.sprites.set(entity.id, drawn);
        }
        const { sprite, shadow } = drawn;
        if (sprite.texture.key !== key) sprite.setTexture(key);
        let scale = actor
          ? 0.6
          : entity.kind === "coin"
            ? 0.21
            : entity.kind === "case"
              ? 0.52
              : entity.kind === "key"
                ? 0.55
                : entity.kind === "vendor"
                  ? 0.85
                  : 0.62;
        if (entity.kind === "cloth") scale = 0.48;
        sprite
          .setPosition(p.x, p.y)
          .setScale(scale * entity.scale)
          .setDepth(p.y);
        if (actor) {
          const direction = entity.direction;
          const flying = view.mode === "flight" && entity.kind !== "rat";
          const animate =
            entity.gait > 0 && (!flying || entity.kind === "owl" || entity.kind === "unicorn");
          if (animate && !reduced.matches && view.phase === "playing")
            sprite.play(`${entity.kind}-${direction}`, true);
          else {
            sprite.stop();
            sprite.setFrame(direction * WALK_FRAMES);
          }
          if (entity.id === view.player.id && view.mode === "flight")
            sprite.setAngle(view.thrust ? -10 : 8);
          else sprite.setAngle(0);
          sprite.setTint(entity.state === 2 && entity.kind === "unicorn" ? 0xffbddb : 0xffffff);
        } else if (entity.kind === "cloth") sprite.setAngle(entity.variant * 47);
        else if (entity.kind === "case")
          sprite.setTint(
            [0xffffff, 0xc8b7c4, 0xb3c6ba, 0xb5bdd7, 0xd9c3ab, 0xc9afd0][entity.variant] ??
              0xffffff,
          );
        if (entity.kind === "puddle" || entity.kind === "rug" || entity.kind === "cloth")
          sprite.setDepth(p.y - 1000);
        shadow
          ?.setPosition(p.x, p.y + 1)
          .setScale(entity.scale)
          .setDepth(p.y - 0.5);
        if (view.mode === "flight" && shadow) shadow.setVisible(entity.kind === "rat");
      }

      draw(view: GameView) {
        this.last = view;
        const nextKey = `${view.seed}:${view.mode}:${view.roof.id}:${view.revision}`;
        if (nextKey !== this.sceneKey) {
          this.sceneKey = nextKey;
          this.resetScenery(view);
        }
        const paused = view.phase !== "playing";
        if (paused || reduced.matches) this.tweens.pauseAll();
        else this.tweens.resumeAll();
        const camera = this.cameras.main;
        const zoom = this.scale.width < 650 ? 0.86 : 1.05;
        camera.setZoom(
          view.mode === "roof" ? zoom : Math.max(0.8, Math.min(1.12, this.scale.height / 720)),
        );
        const p = this.entityPosition(view.player, view);
        const tx =
          p.x -
          this.scale.width / 2 +
          (view.mode === "flight" ? (this.scale.width / camera.zoom) * 0.2 : 0);
        const ty =
          view.mode === "roof"
            ? p.y - this.scale.height / 2 - (this.scale.height / camera.zoom) * 0.08
            : (this.scale.height / camera.zoom - this.scale.height) / 2;
        camera.setScroll(
          this.snapCamera || reduced.matches ? tx : Phaser.Math.Linear(camera.scrollX, tx, 0.11),
          this.snapCamera || reduced.matches ? ty : Phaser.Math.Linear(camera.scrollY, ty, 0.11),
        );
        this.snapCamera = false;
        const width = this.scale.width / camera.zoom,
          height = this.scale.height / camera.zoom;
        // These layers follow the visual camera only; they cannot affect simulation coordinates.
        const left = camera.scrollX + (this.scale.width - width) / 2,
          top = camera.scrollY + (this.scale.height - height) / 2;
        this.sky.setPosition(left, top).setDisplaySize(width, height);
        this.skyline.forEach((layer, i) => {
          layer.setPosition(left, top).setSize(width, height);
          layer.tilePositionX = reduced.matches ? 0 : camera.scrollX * (0.05 + i * 0.06);
          layer.tilePositionY = 720 - height;
        });
        if (view.mode === "flight") {
          for (const roof of view.platforms)
            if (!this.platforms.has(roof.id))
              this.platforms.set(roof.id, paintFlightRoof(this, roof, view.seed));
          const keep = new Set(view.platforms.map((roof) => roof.id));
          for (const [id, objects] of this.platforms)
            if (!keep.has(id)) {
              objects.forEach((object) => object.destroy());
              this.platforms.delete(id);
            }
        }
        const alive = new Set(view.entities.map((e) => e.id));
        for (const [id, drawn] of this.sprites)
          if (!alive.has(id)) {
            drawn.sprite.destroy();
            drawn.shadow?.destroy();
            this.sprites.delete(id);
          }
        view.entities.forEach((entity) => this.paintEntity(entity, view));
        for (const { image, ground } of this.scenery.clothes)
          image.setAlpha(
            Math.abs(image.x - p.x) < 45 && p.y < ground && ground - p.y < 75 ? 0.28 : 0.97,
          );
        this.pack
          .setVisible(view.jetpack)
          .setPosition(p.x - 11, p.y - 21)
          .setDepth(p.y + 1);
        this.flame
          .setVisible(view.jetpack && view.mode === "flight" && view.thrust)
          .setPosition(p.x - 11, p.y - 5)
          .setDepth(p.y - 1)
          .setScale(0.28, reduced.matches ? 0.35 : 0.27 + Math.sin(view.time * 50) * 0.08);
        this.ring.clear();
        if (view.near && view.phase === "playing") {
          const entity = view.entities.find((e) => e.id === view.near?.id);
          if (entity) {
            const near = this.entityPosition(entity, view);
            this.ring
              .setDepth(near.y - 0.6)
              .lineStyle(2, 0xffdba0, 0.8)
              .strokeEllipse(near.x, near.y + 2, 44, 18);
          }
        }
        if (view.feedback && view.feedback.serial !== this.serial) {
          this.serial = view.feedback.serial;
          const icon =
            view.feedback.icon === "locked"
              ? "key"
              : view.feedback.icon === "pack"
                ? "pack"
                : view.feedback.icon === "cloth"
                  ? "cloth-2"
                  : view.feedback.icon;
          const reward = this.add
            .image(p.x, p.y - 40, icon)
            .setDisplaySize(24, 24)
            .setDepth(100000);
          if (view.feedback.icon === "locked") reward.setTint(0xff7777);
          this.tweens.add({
            targets: reward,
            y: p.y - 82,
            alpha: 0,
            duration: reduced.matches ? 200 : 900,
            onComplete: () => reward.destroy(),
          });
          if (view.feedback.icon === "pack") {
            const vendor = view.entities.find((e) => e.kind === "vendor");
            if (vendor) {
              const from = this.entityPosition(vendor, view);
              const delivery = this.add
                .image(from.x, from.y - 24, "pack")
                .setScale(0.5)
                .setDepth(from.y + 3);
              this.tweens.add({
                targets: delivery,
                y: from.y + 7,
                duration: reduced.matches ? 100 : 550,
                ease: "Bounce.Out",
                onComplete: () => {
                  this.tweens.add({
                    targets: delivery,
                    alpha: 0,
                    delay: 600,
                    duration: 300,
                    onComplete: () => delivery.destroy(),
                  });
                },
              });
            }
          }
        }
      }

      pick(clientX: number, clientY: number) {
        const rect = game.canvas.getBoundingClientRect();
        const x = ((clientX - rect.left) * this.scale.width) / rect.width,
          y = ((clientY - rect.top) * this.scale.height) / rect.height;
        const p = this.cameras.main.getWorldPoint(x, y);
        const point = this.last?.mode === "roof" ? unproject(p) : { x: p.x, y: p.y };
        if (this.last?.mode === "roof") {
          const candidates = this.last.entities.filter(
            (e) =>
              ["key", "food", "coin", "case", "vendor", "watercolor", "paper", "velvet"].includes(
                e.kind,
              ) && !e.used,
          );
          for (const e of candidates.sort((a, b) => b.x + b.y - a.x - a.y)) {
            if (this.sprites.get(e.id)?.sprite.getBounds().contains(p.x, p.y))
              return { point: { x: e.x, y: e.y }, entity: e.id };
          }
        }
        return { point };
      }
    }
    game = new Phaser.Game({
      type: Phaser.AUTO,
      parent: host,
      width: host.clientWidth,
      height: host.clientHeight,
      backgroundColor: "#101322",
      banner: false,
      canvasStyle: "display:block;",
      audio: { noAudio: true },
      input: { keyboard: false, mouse: false, touch: false, gamepad: false },
      scale: { mode: Phaser.Scale.RESIZE, autoCenter: Phaser.Scale.NO_CENTER },
      render: { antialias: true },
      scene: [ViewScene],
    });
  });
}
