export type PlayerState = {
  title: string;
  artist: string;
  album: string;
  position: string;
  duration: string;
  progressPercent: number;
  artworkUrl: string;
  songUrl: string;
  isPlaying: boolean;
};

export type SessionTrack = {
  title: string;
  artist: string;
  album: string;
  duration: string;
  artworkUrl: string;
  songUrl: string;
  firstSeenAt: string;
  lastSeenAt: string;
  playCount: number;
};

export type AppSettings = {
  overlayPort: number;
};
