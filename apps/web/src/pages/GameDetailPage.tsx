import { A } from "@solidjs/router";
import Navbar from "../components/Navbar";
import Footer from "../components/Footer";

export default function GameDetailPage() {
  return (
    <div>
      <Navbar activeLink="games" />
      <GameHero />
      <LobbySection />
      <Footer />
    </div>
  );
}

function GameHero() {
  return (
    <section class="game-hero">
      <div class="game-hero-image">
        {/* Placeholder gradient for hero image */}
      </div>

      <div class="game-hero-content">
        <div class="breadcrumb">
          <A href="/" class="breadcrumb-parent">Games</A>
          <span class="breadcrumb-sep">›</span>
          <span class="breadcrumb-current">Chess</span>
        </div>

        <div class="game-title-row">
          <div class="game-emoji">♟</div>
          <h1 class="game-page-title">Chess</h1>
        </div>

        <div class="game-tags">
          <span class="game-tag">Classic</span>
          <span class="game-tag">Strategy</span>
          <span class="game-tag">2 Players</span>
        </div>

        <p class="game-desc">
          The timeless game of kings — over 1,500 years of history, infinite
          strategic depth. Battle opponents in real-time, explore variants like
          Atomic or Crazyhouse, and climb the global leaderboard.
        </p>
      </div>
    </section>
  );
}

function LobbySection() {
  return (
    <section class="lobby-section">
      <div class="lobby-main">
        <div class="lobby-header">
          <h2 class="lobby-title">Join a Game</h2>
          <button class="btn-create-game">
            <span>+</span>
            <span>Create Game</span>
          </button>
        </div>

        <span class="open-games-label">OPEN GAMES</span>

        <GameRow
          name="Magnus_K"
          meta="Blitz 3+2  ·  Elo ~1850"
          elo="1850"
          eloColor="#FF6B35"
          eloBg="#FFF3E8"
          avatarColor="#FF6B35"
        />
        <GameRow
          name="Anna_P"
          meta="Rapid 10+0  ·  Elo ~1620"
          elo="1620"
          eloColor="#4A6AFF"
          eloBg="#EEF0FF"
          avatarColor="#4A6AFF"
        />
        <GameRow
          name="Rook_Junior"
          meta="Classical 30+0  ·  Elo ~1340"
          elo="1340"
          eloColor="#22A055"
          eloBg="#EDF9F2"
          avatarColor="#22A055"
        />
      </div>

      <div class="lobby-sidebar">
        <CommunityCard />
        <QuickPlayCard />
      </div>
    </section>
  );
}

interface GameRowProps {
  name: string;
  meta: string;
  elo: string;
  eloColor: string;
  eloBg: string;
  avatarColor: string;
}

function GameRow(props: GameRowProps) {
  return (
    <div class="game-row">
      <div class="game-row-left">
        <div
          class="game-row-avatar"
          style={{ background: props.avatarColor }}
        />
        <div class="game-row-info">
          <span class="game-row-name">{props.name}</span>
          <span class="game-row-meta">{props.meta}</span>
        </div>
      </div>
      <div class="game-row-right">
        <span
          class="elo-badge"
          style={{ background: props.eloBg, color: props.eloColor }}
        >
          {props.elo}
        </span>
        <button class="btn-join">Join</button>
      </div>
    </div>
  );
}

function CommunityCard() {
  return (
    <div class="community-card">
      <h3 class="community-card-title">Chess Community</h3>
      <div class="comm-stats">
        <div class="comm-stat-row">
          <span class="comm-stat-label">Members</span>
          <span class="comm-stat-value">48,291</span>
        </div>
        <div class="comm-stat-row">
          <span class="comm-stat-label">Online now</span>
          <div class="comm-stat-online">
            <div class="online-dot" />
            <span class="comm-stat-value">1,847</span>
          </div>
        </div>
        <div class="comm-stat-row">
          <span class="comm-stat-label">Games today</span>
          <span class="comm-stat-value">23,412</span>
        </div>
      </div>
      <button class="btn-join-community">Join Community</button>
    </div>
  );
}

function QuickPlayCard() {
  return (
    <div class="quick-play-card">
      <h3 class="quick-play-title">Quick Play</h3>
      <p class="quick-play-sub">
        Get matched instantly against a player near your skill level.
      </p>
      <div class="time-controls">
        <div class="time-control">
          <span class="time-control-time">3+2</span>
          <span class="time-control-label">Blitz</span>
        </div>
        <div class="time-control active">
          <span class="time-control-time">10+0</span>
          <span class="time-control-label">Rapid</span>
        </div>
        <div class="time-control">
          <span class="time-control-time">30+0</span>
          <span class="time-control-label">Classical</span>
        </div>
      </div>
      <button class="btn-play-now">
        <span>▶</span>
        <span>Play Now</span>
      </button>
    </div>
  );
}
