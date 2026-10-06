import json
import os
import shutil
import subprocess
import sys

RAW_DIR = "/home/gulshansharma/proof_battle/recordings_raw"
DOCS_DIR = "/home/gulshansharma/proof_battle/docs"
SCREENSHOTS_DIR = "/home/gulshansharma/proof_battle/screenshots"
ARTIFACT_DIR = "/home/gulshansharma/.gemini/antigravity/brain/55900e62-6dd0-4a66-9d63-cbb51624059e"

BOLD_FONT = "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf"
REG_FONT = "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf"

def run_cmd(cmd):
    print("Running:", " ".join(cmd))
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if res.returncode != 0:
        print("Error executing command:", res.stderr)
        sys.exit(res.returncode)
    return res

def get_duration(video_path):
    cmd = [
        "ffprobe", "-v", "error",
        "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        video_path
    ]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return float(res.stdout.strip())

def main():
    chapters_path = os.path.join(RAW_DIR, "chapters.json")
    with open(chapters_path, "r", encoding="utf-8") as f:
        meta = json.load(f)

    webm_files = [f for f in os.listdir(RAW_DIR) if f.endswith(".webm")]
    if not webm_files:
        print("No raw webm found in", RAW_DIR)
        sys.exit(1)
    raw_video = os.path.join(RAW_DIR, webm_files[0])
    raw_duration = get_duration(raw_video)
    print(f"Using raw video: {raw_video} (duration: {raw_duration:.2f}s)")

    os.makedirs(DOCS_DIR, exist_ok=True)
    os.makedirs(SCREENSHOTS_DIR, exist_ok=True)
    os.makedirs(ARTIFACT_DIR, exist_ok=True)

    intro_clip = "/tmp/intro_card.mp4"
    outro_clip = "/tmp/outro_card.mp4"
    overlayed_clip = "/tmp/walkthrough_overlayed.mp4"
    final_output = os.path.join(DOCS_DIR, "proofbattle_presentation_showcase.mp4")

    print("Step 1: Generating Intro Title Slate (3.5s)...")
    intro_filters = (
        f"drawbox=x=0:y=0:w=1920:h=1080:color=0x090d16@1.0:t=fill,"
        f"drawbox=x=160:y=420:w=12:h=230:color=0x38bdf8@1.0:t=fill,"
        f"drawtext=fontfile={BOLD_FONT}:text='PROOFBATTLE':fontsize=76:fontcolor=0xffffff:x=200:y=430,"
        f"drawtext=fontfile={REG_FONT}:text='Real-Time Competitive Formal Mathematics in Lean 4':fontsize=36:fontcolor=0x38bdf8:x=200:y=525,"
        f"drawtext=fontfile={REG_FONT}:text='1v1 Arena - Monaco Editor - Mathlib Library - Automated Lean 4 Judge':fontsize=24:fontcolor=0x94a3b8:x=200:y=590,"
        f"fade=t=in:st=0:d=0.5,fade=t=out:st=3.0:d=0.5"
    )
    run_cmd([
        "ffmpeg", "-y",
        "-f", "lavfi", "-i", "color=c=0x090d16:s=1920x1080:d=3.5:r=30",
        "-vf", intro_filters,
        "-c:v", "libopenh264", "-b:v", "5M", "-pix_fmt", "yuv420p",
        intro_clip
    ])

    print("Step 2: Generating Outro Closing Slate (3.5s)...")
    outro_filters = (
        f"drawbox=x=0:y=0:w=1920:h=1080:color=0x090d16@1.0:t=fill,"
        f"drawbox=x=(w-800)/2:y=380:w=800:h=4:color=0x38bdf8@0.8:t=fill,"
        f"drawtext=fontfile={BOLD_FONT}:text='PROOFBATTLE':fontsize=68:fontcolor=0xffffff:x=(w-text_w)/2:y=420,"
        f"drawtext=fontfile={REG_FONT}:text='Machine-Checked Mathematics Competitions':fontsize=32:fontcolor=0x38bdf8:x=(w-text_w)/2:y=510,"
        f"drawtext=fontfile={REG_FONT}:text='Architecture - Rust Axum - Lean 4 Toolchain - SvelteKit 2 - Tailwind CSS':fontsize=24:fontcolor=0x94a3b8:x=(w-text_w)/2:y=570,"
        f"drawbox=x=(w-800)/2:y=630:w=800:h=4:color=0x38bdf8@0.8:t=fill,"
        f"fade=t=in:st=0:d=0.5,fade=t=out:st=3.0:d=0.5"
    )
    run_cmd([
        "ffmpeg", "-y",
        "-f", "lavfi", "-i", "color=c=0x090d16:s=1920x1080:d=3.5:r=30",
        "-vf", outro_filters,
        "-c:v", "libopenh264", "-b:v", "5M", "-pix_fmt", "yuv420p",
        outro_clip
    ])

    print("Step 3: Building Chapter Overlays Filtergraph...")
    overlay_filters = ["fps=30"]
    chapters = meta.get("chapters", [])
    for idx, ch in enumerate(chapters):
        st = ch["start"]
        et = min(ch["end"], raw_duration)
        num = f"{idx+1:02d}/{len(chapters):02d}"
        title = ch["title"].replace("'", "").replace(":", "-").replace(",", " -")
        subtitle = ch["subtitle"].replace("'", "").replace(":", "-").replace(",", " -")

        # Box backdrop
        overlay_filters.append(
            f"drawbox=x=36:y=18:w=680:h=56:color=0x070c18@0.92:t=fill:enable='between(t,{st:.3f},{et:.3f})'"
        )
        # Left cyan accent line
        overlay_filters.append(
            f"drawbox=x=36:y=18:w=5:h=56:color=0x38bdf8@1.0:t=fill:enable='between(t,{st:.3f},{et:.3f})'"
        )
        # Category tag
        overlay_filters.append(
            f"drawtext=fontfile={BOLD_FONT}:text='FEATURE {num}':fontsize=15:fontcolor=0x38bdf8:x=56:y=24:enable='between(t,{st:.3f},{et:.3f})'"
        )
        # Chapter title
        overlay_filters.append(
            f"drawtext=fontfile={BOLD_FONT}:text='{title}':fontsize=18:fontcolor=0xffffff:x=200:y=23:enable='between(t,{st:.3f},{et:.3f})'"
        )
        # Chapter subtitle
        overlay_filters.append(
            f"drawtext=fontfile={REG_FONT}:text='{subtitle}':fontsize=14:fontcolor=0x94a3b8:x=56:y=48:enable='between(t,{st:.3f},{et:.3f})'"
        )

    vf_chain = ",".join(overlay_filters)
    print("Step 4: Rendering Overlays on Main Footage...")
    run_cmd([
        "ffmpeg", "-y",
        "-i", raw_video,
        "-vf", vf_chain,
        "-c:v", "libopenh264", "-b:v", "5M", "-pix_fmt", "yuv420p",
        overlayed_clip
    ])

    overlayed_duration = get_duration(overlayed_clip)
    total_video_duration = 3.5 + overlayed_duration + 3.5
    print(f"Total compiled video duration will be: {total_video_duration:.2f}s")

    print("Step 5: Concatenating [Intro + Walkthrough + Outro] with Filter Graph & Audio...")
    concat_filter = "[0:v][1:v][2:v]concat=n=3:v=1:a=0[outv]"
    run_cmd([
        "ffmpeg", "-y",
        "-i", intro_clip,
        "-i", overlayed_clip,
        "-i", outro_clip,
        "-f", "lavfi", "-i", f"anullsrc=channel_layout=stereo:sample_rate=44100:d={total_video_duration:.2f}",
        "-filter_complex", concat_filter,
        "-map", "[outv]",
        "-map", "3:a",
        "-c:v", "libopenh264", "-b:v", "5M", "-pix_fmt", "yuv420p",
        "-c:a", "aac", "-b:a", "128k",
        "-shortest",
        "-movflags", "+faststart",
        final_output
    ])

    print(f"Final presentation video successfully created at: {final_output}")

    # Copy to screenshots/ and artifact directory
    screenshots_target = os.path.join(SCREENSHOTS_DIR, "proofbattle_presentation_showcase.mp4")
    artifact_target = os.path.join(ARTIFACT_DIR, "proofbattle_presentation_showcase.mp4")

    shutil.copy2(final_output, screenshots_target)
    shutil.copy2(final_output, artifact_target)
    print(f"Copied to: {screenshots_target}")
    print(f"Copied to: {artifact_target}")

    # Generate high-quality GIF preview (battle arena through victory modal)
    gif_output = os.path.join(ARTIFACT_DIR, "proofbattle_preview.gif")
    gif_screenshots = os.path.join(SCREENSHOTS_DIR, "proofbattle_preview.gif")
    print("Generating lightweight preview GIF for presentation slides...")
    run_cmd([
        "ffmpeg", "-y",
        "-ss", "16.0", "-t", "14.0", "-i", final_output,
        "-vf", "fps=10,scale=800:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer",
        gif_output
    ])
    shutil.copy2(gif_output, gif_screenshots)
    print(f"Preview GIF saved to: {gif_output} and {gif_screenshots}")

if __name__ == "__main__":
    main()
