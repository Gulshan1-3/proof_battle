#!/usr/bin/env python3
import asyncio
import json
import websockets

URI = "ws://127.0.0.1:4000/ws"

async def run_demo():
    print("=" * 65)
    print("       ⚔️   PROOFBATTLE ENGINE: LIVE DEMONSTRATION   ⚔️")
    print("=" * 65)
    print(f"Connecting 2 players ('Alice' and 'Bob') to WebSocket server ({URI})...\n")

    async with websockets.connect(URI) as ws_alice, websockets.connect(URI) as ws_bob:
        # 1. Handshake
        print("── 1. PROTOCOL HANDSHAKE (Hello -> Welcome + ServerTime) ────────")
        await ws_alice.send(json.dumps({
            "type": "Hello",
            "version": 1,
            "token": None,
            "username": "Alice"
        }))
        alice_welcome = json.loads(await ws_alice.recv())
        alice_server_time = json.loads(await ws_alice.recv())
        print(f"  [Alice] ID:            {alice_welcome['player_id']}")
        print(f"  [Alice] Session Token: {alice_welcome['session_token']}")
        print(f"  [Alice] Server Time:   {alice_server_time['server_time_ms']} ms")

        await ws_bob.send(json.dumps({
            "type": "Hello",
            "version": 1,
            "token": None,
            "username": "Bob"
        }))
        bob_welcome = json.loads(await ws_bob.recv())
        bob_server_time = json.loads(await ws_bob.recv())
        print(f"  [Bob]   ID:            {bob_welcome['player_id']}")
        print(f"  [Bob]   Session Token: {bob_welcome['session_token']}\n")

        # 2. Queue Join
        print("── 2. MATCHMAKING QUEUE (QueueJoin -> QueueStatus) ──────────────")
        await ws_alice.send(json.dumps({"type": "QueueJoin"}))
        alice_q = json.loads(await ws_alice.recv())
        print(f"  [Alice] Queued! queue_size={alice_q['queue_size']}, search_range={alice_q['search_range']['min_elo']} - {alice_q['search_range']['max_elo']}")

        await ws_bob.send(json.dumps({"type": "QueueJoin"}))
        bob_q = json.loads(await ws_bob.recv())
        print(f"  [Bob]   Queued! queue_size={bob_q['queue_size']}, search_range={bob_q['search_range']['min_elo']} - {bob_q['search_range']['max_elo']}\n")

        # 3. Match Found & Round Start
        print("── 3. CLOSEST-PAIR ELO MATCHMAKING & ROUND DISPATCH ─────────────")
        alice_match = json.loads(await ws_alice.recv())
        bob_match = json.loads(await ws_bob.recv())
        print(f"  Room ID:        {alice_match['room_id']}")
        print(f"  Alice Opponent: {alice_match['opponent']['username']} (Elo: {alice_match['opponent']['elo']})")
        print(f"  Bob Opponent:   {bob_match['opponent']['username']} (Elo: {bob_match['opponent']['elo']})\n")

        alice_round = json.loads(await ws_alice.recv())
        bob_round = json.loads(await ws_bob.recv())
        problem = alice_round['problem']
        print(f"  📜 Theorem Goal:   {problem['goal']}")
        print(f"  📦 Fine Imports:   {problem['imports']}")
        print(f"  ⭐ Difficulty:     {problem['difficulty']}")
        print(f"  ⏱  Round Duration: {alice_round['duration_ms'] / 1000:.0f}s\n")

        # 4. Alice attempts to cheat with `sorry`
        print("── 4. STAGE S2 AST ANTI-CHEAT FILTER (Alice submits 'sorry') ───")
        await ws_alice.send(json.dumps({
            "type": "ProofRequest",
            "req_id": "req-cheat",
            "code": "sorry",
            "intent": "Submit"
        }))
        alice_verdict_1 = json.loads(await ws_alice.recv())
        print(f"  Verdict:  {alice_verdict_1['verdict']}")
        print(f"  Reason:   {alice_verdict_1['reason']}")
        print(f"  Message:  {alice_verdict_1['message']}\n")

        # 5. Alice submits invalid tactic syntax
        print("── 5. LIVE LEAN 4 DIAGNOSTIC REPORTING (Alice syntax error) ─────")
        await ws_alice.send(json.dumps({
            "type": "ProofRequest",
            "req_id": "req-syntax",
            "code": "intro h\nfake_tactic_xyz",
            "intent": "Check"
        }))
        alice_verdict_2 = json.loads(await ws_alice.recv())
        print(f"  Intent:   Check (Diagnostic check-lane execution)")
        print(f"  Verdict:  {alice_verdict_2['verdict']}")
        print(f"  Reason:   {alice_verdict_2['reason']}")
        print(f"  Message:  {alice_verdict_2['message']}\n")

        # 6. Bob solves the problem correctly
        print("── 6. REAL LEAN 4 PROOF COMPILATION & VERIFICATION (Bob) ───────")
        goal = problem['goal']
        if "P ∧ Q" in goal:
            solution = "intro P Q h\nexact ⟨h.2, h.1⟩"
        elif "a + b = b + a" in goal:
            solution = "intro a b\nsimp [Nat.add_comm]"
        else:
            solution = "intro n\nsimp"

        print(f"  Bob tactic body:\n    {solution.replace(chr(10), chr(10) + '    ')}")
        await ws_bob.send(json.dumps({
            "type": "ProofRequest",
            "req_id": "req-solve",
            "code": solution,
            "intent": "Submit"
        }))

        bob_verdict = json.loads(await ws_bob.recv())
        print(f"  Verdict:      {bob_verdict['verdict']}")
        print(f"  Verify Time:  {bob_verdict['elapsed_ms']} ms\n")

        # 7. Match Outcome for both players
        print("── 7. STAGE S6 GAME COMPLETION & ELO RESOLUTION ─────────────────")
        alice_end = json.loads(await ws_alice.recv())
        bob_end = json.loads(await ws_bob.recv())

        print(f"  [Alice View] Outcome: {alice_end['outcome']} | Elo Change: {alice_end['elo_delta']}")
        print(f"  [Bob View]   Outcome: {bob_end['outcome']} | Elo Change: {bob_end['elo_delta']}")
        print(f"  Winner ID:      {bob_end['winner_id']}")
        print(f"  Winning Proof:  {repr(bob_end['winning_proof'])}")
        print(f"  Canonical Sol:  {repr(bob_end['canonical_proof'])}")
        print(f"  Match Duration: {bob_end['duration_ms']} ms\n")
        print("=" * 65)
        print(" 🎉 FULL MATCH ENGINE DEMO COMPLETED SUCCESSFULLY! ")
        print("=" * 65)

if __name__ == "__main__":
    asyncio.run(run_demo())
