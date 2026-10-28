import { loadHouseArtManifest } from "@paper-theater/player";
import type { LoadedHouseArt, HouseArtManifest } from "@paper-theater/player";

const DEV_ASSETS: HouseArtManifest["assets"] = {
  "aisle.row-a": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/aisle-row-a.png",
    sha256: "9fdfe4bd0ac4b64bc1dcd8af17649c0a6565336c6fcfc666c544a5aafa782e9c",
    deliver_px: [1536, 597],
    alpha: "cutout",
  },
  "aisle.row-b": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/aisle-row-b.png",
    sha256: "411467014645ecc473e476625849392ce98c6647da28aa70fca905a57d95ddbd",
    deliver_px: [1536, 597],
    alpha: "cutout",
  },
  "floor.carpet": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/floor-carpet.png",
    sha256: "7d1122e0caa4daa459544cb7b118c8e0ce8d579cc83ed40c9189f8c9e4ba71ba",
    deliver_px: [512, 512],
    alpha: "tile",
  },
  "floor.runner": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/floor-runner.png",
    sha256: "74c88f300dc253ae990622f09e5c1e66a8fbd5e9b2d8130518b34964e47de323",
    deliver_px: [256, 512],
    alpha: "tileY",
  },
  "house.curtain": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house-curtain.png",
    sha256: "a848c2fe2e11063e1254c8587d825d4044146a2c3a5ac0ac8fb33ceb803cb58b",
    deliver_px: [768, 864],
    alpha: "tileX",
  },
  "house.rope": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house-rope.png",
    sha256: "3ca738286eec4f93304837dac9dd9b71cac85a6716d32bf54fff2e74b641c93c",
    deliver_px: [64, 512],
    alpha: "tileY",
  },
  "house.sign-board": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house-sign-board.png",
    sha256: "6ac4176d1c4fc900c4e9be988971dfe18fa41cc06d7a043532661806113d851b",
    deliver_px: [1024, 256],
    alpha: "opaque",
  },
  "house.title-board": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house-title-board.png",
    sha256: "05db70e2deca3929530650eb52006189a9efa8ba5cdba74d020877755c037448",
    deliver_px: [1024, 319],
    alpha: "opaque",
  },
  "house.wall": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house-wall.png",
    sha256: "7f483332a96a215259fd04f848ceada3d01c95ec5b702974852f89d5bd4ef13c",
    deliver_px: [512, 512],
    alpha: "tile",
  },
  "house.facade": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/house.facade-v2.png",
    sha256: "247fba44cfd088e1b6d8272ca272e335e38bbbea85566ba1554365147490b40c",
    deliver_px: [2048, 1245],
    alpha: "aperture",
  },
  "lobby.archway": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-archway.png",
    sha256: "8470ee112dbfdb49a14da0d5962ac4cf730a4db7212841ce38ccfd50aa2f5b4e",
    deliver_px: [768, 1220],
    alpha: "aperture",
  },
  "lobby.booth-surround": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-booth-surround.png",
    sha256: "6bb3128c05640a28b96248182761d0bd7b49b882361f05ebc00d09de98c7ce9f",
    deliver_px: [512, 512],
    alpha: "tile",
  },
  "lobby.counter": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-counter.png",
    sha256: "afdbaddcde4fcf4b0c8339df7834703ee521344d4b28396c02786436e384ef5e",
    deliver_px: [1280, 672],
    alpha: "tileX",
  },
  "lobby.doors": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-doors.png",
    sha256: "f54400905521bcb6959a7d11d023a09ff9e32d7c314331aab38f15b5b299195e",
    deliver_px: [1024, 1516],
    alpha: "opaque",
  },
  "lobby.drape-left": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-drape-left.png",
    sha256: "da61a3aa154785ab5799ccc2ddf1e5cf4dbcba0189a81f3377fa10a7f9607c71",
    deliver_px: [352, 2112],
    alpha: "cutout",
  },
  "lobby.drape-right": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-drape-right.png",
    sha256: "91ce0ba3031ffdcfd868b2050fbfe2cfeb86d9536f830cec8ad85d2766a7bf03",
    deliver_px: [352, 2112],
    alpha: "cutout",
  },
  "lobby.grille": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-grille.png",
    sha256: "a5d5bb94987279281844be9346e0292681ab2593e5a78d879f34d4c8c8b1a578",
    deliver_px: [2560, 1792],
    alpha: "aperture",
  },
  "lobby.inspect-card": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-inspect-card.png",
    sha256: "ec649e2a80dcb343620197050a75a7f2e3f2e07b7d5d076d8f50dd1763c8c807",
    deliver_px: [1664, 1024],
    alpha: "opaque",
  },
  "lobby.playbill-a": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-playbill-a.png",
    sha256: "35ffa972797767edef6f697448a1aef4a8539a9d777834767349d4d986e9a4ba",
    deliver_px: [640, 960],
    alpha: "opaque",
  },
  "lobby.playbill-b": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-playbill-b.png",
    sha256: "1f2d0f4392d93f19237fd48e28458245459be1882cff1f060fb29ec7206b97a1",
    deliver_px: [640, 960],
    alpha: "opaque",
  },
  "lobby.palm": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-palm.png",
    sha256: "d718072439167de2a3eda7c26b6b413080fa719510f6775c27b9baa0c7d937f0",
    deliver_px: [864, 1152],
    alpha: "cutout",
  },
  "lobby.sconce": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-sconce.png",
    sha256: "fa5c038c9538866434d7540b017bb3ca201eb67b59ba1037f7b6deb389517773",
    deliver_px: [384, 384],
    alpha: "cutout",
  },
  "lobby.ticket": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-ticket.png",
    sha256: "17f18566a710c60ce754f02e2e77586924da44ad2f0066643d7e36d38f96241f",
    deliver_px: [1024, 529],
    alpha: "opaque",
  },
  "lobby.wallpaper": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/lobby-wallpaper.png",
    sha256: "061060b43415387ce57c2f47b9389ee197258964649f1695fbec4b2f82dee85a",
    deliver_px: [512, 512],
    alpha: "tile",
  },
  "shared.brass-plate": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/shared-brass-plate.png",
    sha256: "ab8dc54c8e38dbb2d2b9fa2c8a5d01f077d99b6dad5bcb20e83abb6ea3481ea6",
    deliver_px: [1024, 209],
    alpha: "opaque",
  },
  "banner.play": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/banner-play.png",
    sha256: "555a580f81ee3e1a5e02f2908331559d2dad2239ad1bf97df21de25da36ca5ce",
    deliver_px: [512, 512],
    alpha: "cutout",
  },
  "banner.restart": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/banner-restart.png",
    sha256: "e56575e14e915fefc43f7f8282880d980279b4bc38eae31a26902550a4d1090b",
    deliver_px: [512, 512],
    alpha: "cutout",
  },
  "banner.readalong": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/banner-readalong.png",
    sha256: "1d8e184ae2612d1b82698f44237d0aeb72516fdc63b37452669392cc23b67b9e",
    deliver_px: [512, 512],
    alpha: "cutout",
  },
  "banner.return": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/banner-return.png",
    sha256: "434f5726b8dc1ade2c83e526da73f86a5534d38d1f0c786d07c64f01af544f0c",
    deliver_px: [512, 512],
    alpha: "cutout",
  },
  "banner.seek": {
    url: "/@fs/Users/heycato/Projects/paper-theater/fixtures/generation/immersive-house-test-v2/banner-seek.png",
    sha256: "246e713612c29c7683f61230c6094cfeaee05d8bbd9d5634fb3a36eab8865136",
    deliver_px: [1280, 240],
    alpha: "cutout",
  },
};

export async function loadDevHouseArt(): Promise<LoadedHouseArt | null> {
  try {
    const manifest: HouseArtManifest = {
      schema_version: 1,
      brief: "immersive-house-art-v1",
      diagnostic: true,
      accepted: false,
      assets: DEV_ASSETS,
    };
    const manifestBytes = new TextEncoder().encode(JSON.stringify(manifest));
    const read = async (url: string) => {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`fetch failed: ${res.status} for ${url}`);
      return new Uint8Array(await res.arrayBuffer());
    };
    return await loadHouseArtManifest(manifestBytes, read);
  } catch (err) {
    console.warn("Could not load dev diagnostic house art:", err);
    return null;
  }
}
