import { A } from "@solidjs/router";
import Navbar from "../components/Navbar";
import Footer from "../components/Footer";

export default function HomePage() {
  return (
    <div>
      <Navbar />
      <Hero />
      <ChessStrip />
      <GameFeed />
      <StatsBar />
      <Footer />
    </div>
  );
}

function Hero() {
  return (
    <section class="hero">
      <div class="hero-badge">
        <div class="hero-badge-dot" />
        <span class="hero-badge-text">✨ 60+ games available now</span>
      </div>

      <h1 class="hero-title">
        The Hub for{"\n"}Every Board Game
      </h1>

      <p class="hero-sub">
        Discover, play, and build communities around classic and modern board
        games. From chess to Shogi — it all starts here.
      </p>

      <div class="hero-ctas">
        <A href="/" class="btn-primary-lg">Explore Games</A>
        <button class="btn-secondary-lg">How it works</button>
      </div>

      <div class="hero-pills">
        <span class="hero-pill">♟ Chess</span>
        <span class="hero-pill" style={{ background: "#001E29", color: "#4ABDE0" }}>将 Shogi</span>
        <span class="hero-pill" style={{ background: "#001A0D", color: "#42C97A" }}>⬡ Go</span>
        <span class="hero-pill" style={{ background: "#150A2E", color: "#B882F5" }}>◉ Ataxx</span>
        <span class="hero-pill muted">+ more</span>
      </div>
    </section>
  );
}

function ChessStrip() {
  return (
    <div class="chess-strip">
      <span class="chess-strip-text">
        ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟
      </span>
    </div>
  );
}

function GameFeed() {
  return (
    <section class="game-feed">
      <div class="feed-header">
        <div class="feed-title-group">
          <span class="feed-eyebrow">✦ DISCOVER ✦</span>
          <h2 class="feed-title">Explore Games</h2>
        </div>
        <div class="feed-filters">
          <span class="filter-pill active">All</span>
          <span class="filter-pill">Strategy</span>
          <span class="filter-pill">Abstract</span>
          <span class="filter-pill">Classic</span>
        </div>
      </div>

      <div class="game-grid">
        <A href="/games/chess" class="featured-card">
          <div class="featured-card-img">
            <span class="featured-badge">🏆 FEATURED</span>
          </div>
          <div class="featured-card-info">
            <h3 class="featured-card-title">Chess</h3>
            <p class="featured-card-desc">
              The timeless game of kings. Battle wits in classic or variant
              formats, climb the ladder, and join a thriving community.
            </p>
            <div class="rating-row">
              <span class="rating-stars">★★★★★</span>
              <span class="rating-text">4.9 · 12.4K reviews</span>
            </div>
            <div class="featured-card-meta">
              <span class="featured-players">
                <span class="featured-players-icon">👥</span> 48.2K playing
              </span>
              <button class="btn-play">Play now</button>
            </div>
          </div>
        </A>

        <div class="small-cards-col">
          <SmallCard
            href="/games/shogi"
            title="Shogi"
            desc="Japanese chess with drops"
            players="12.1K playing"
            genre="♟ Strategy"
            imgUrl="https://images.unsplash.com/photo-1688381691547-36831dd5f435?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
          />
          <SmallCard
            href="/games/go"
            title="Go"
            desc="Ancient art of territorial conquest"
            players="31.8K playing"
            genre="⬡ Territory"
            imgUrl="https://images.unsplash.com/photo-1748122851298-db69fcfef1b5?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
          />
          <SmallCard
            href="/games/racing-kings"
            title="Racing Kings"
            desc="Race your king to the 8th rank"
            players="8.3K playing"
            genre="♚ Race"
            imgUrl="https://images.unsplash.com/photo-1628867578000-236840cc19d3?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
          />
        </div>
      </div>

      <div class="row2-cards">
        <div class="row2-card" style={{ background: "#1A0A00" }}>
          <div
            class="row2-card-img-placeholder"
            style={{ background: "linear-gradient(135deg, #5a2a0a, #8B4513)" }}
          />
          <div class="row2-card-info">
            <span class="row2-card-title" style={{ color: "white" }}>Atomic Chess</span>
            <span class="row2-card-desc" style={{ color: "#A07A5A" }}>
              Captures explode — pieces fly
            </span>
            <span class="row2-card-badge">HOT 🔥</span>
          </div>
        </div>

        <div class="row2-card" style={{ background: "#1A0A00" }}>
          <img
            class="row2-card-img"
            src="https://images.unsplash.com/photo-1603290989627-5155a7f83dac?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
            alt="Crazyhouse"
          />
          <div class="row2-card-info">
            <span class="row2-card-title" style={{ color: "white" }}>Crazyhouse</span>
            <span class="row2-card-desc" style={{ color: "#A07A5A" }}>
              Drop captured pieces back into play
            </span>
            <span class="row2-card-badge">✦ NEW</span>
          </div>
        </div>

        <div class="row2-card" style={{ background: "#1A0A00" }}>
          <img
            class="row2-card-img"
            src="https://images.unsplash.com/photo-1645541636667-becfee706050?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
            alt="King of the Hill"
          />
          <div class="row2-card-info">
            <span class="row2-card-title" style={{ color: "white" }}>King of the Hill</span>
            <span class="row2-card-desc" style={{ color: "#A07A5A" }}>
              Control the center to win
            </span>
            <span class="row2-card-badge" style={{ background: "#2A1A00", color: "#FF9A5C" }}>★ Classic</span>
          </div>
        </div>

        <div class="row2-card" style={{ background: "#1A0A00" }}>
          <img
            class="row2-card-img"
            src="https://images.unsplash.com/photo-1655721529468-d0d81b2dc489?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&w=400"
            alt="Ataxx"
          />
          <div class="row2-card-info">
            <span class="row2-card-title" style={{ color: "white" }}>Ataxx</span>
            <span class="row2-card-desc" style={{ color: "#A07A5A" }}>
              Clone and conquer the board
            </span>
          </div>
        </div>
      </div>
    </section>
  );
}

interface SmallCardProps {
  href: string;
  title: string;
  desc: string;
  players: string;
  imgUrl: string;
  genre?: string;
}

function SmallCard(props: SmallCardProps) {
  return (
    <A href={props.href} class="small-card">
      <img class="small-card-img" src={props.imgUrl} alt={props.title} />
      <div class="small-card-info">
        <span class="small-card-title">{props.title}</span>
        <span class="small-card-desc">{props.desc}</span>
        <span class="small-card-players">{props.players}</span>
        {props.genre && <span class="genre-tag">{props.genre}</span>}
      </div>
    </A>
  );
}

function StatsBar() {
  return (
    <section class="stats-bar">
      <div class="stat-item">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#FFE0C8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/>
          <circle cx="9" cy="7" r="4"/>
          <path d="M22 21v-2a4 4 0 0 0-3-3.87"/>
          <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
        </svg>
        <span class="stat-number">192K+</span>
        <span class="stat-label">Active Players</span>
      </div>
      <div class="stat-divider" />
      <div class="stat-item">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#FFE0C8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="3" width="7" height="7" rx="1"/>
          <rect x="14" y="3" width="7" height="7" rx="1"/>
          <rect x="14" y="14" width="7" height="7" rx="1"/>
          <rect x="3" y="14" width="7" height="7" rx="1"/>
        </svg>
        <span class="stat-number">60+</span>
        <span class="stat-label">Unique Games</span>
      </div>
      <div class="stat-divider" />
      <div class="stat-item">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#FFE0C8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"/>
          <path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"/>
          <path d="M4 22h16"/>
          <path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22"/>
          <path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22"/>
          <path d="M18 2H6v7a6 6 0 0 0 12 0V2Z"/>
        </svg>
        <span class="stat-number">4.2M</span>
        <span class="stat-label">Games Played</span>
      </div>
      <div class="stat-divider" />
      <div class="stat-item">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#FFE0C8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>
        </svg>
        <span class="stat-number">850+</span>
        <span class="stat-label">Communities</span>
      </div>
    </section>
  );
}
