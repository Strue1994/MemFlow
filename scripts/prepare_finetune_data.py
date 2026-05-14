"""Prepare fine-tuning data from feedback logs."""

import json
from pathlib import Path
from typing import List, Dict


def load_feedback(file_path: str = "data/feedback.jsonl") -> List[Dict]:
    """Load feedback records from file."""
    data = []
    path = Path(file_path)
    
    if not path.exists():
        print(f"File not found: {file_path}")
        return data
    
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                try:
                    record = json.loads(line)
                    if record.get("feedback") == "positive":
                        data.append(record)
                except json.JSONDecodeError:
                    continue
    
    return data


def save_as_jsonl(data: List[Dict], output_path: str) -> None:
    """Save data as JSONL format."""
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(path, "w", encoding="utf-8") as f:
        for item in data:
            f.write(json.dumps(item, ensure_ascii=False) + "\n")


def prepare_train_val_split(
    data: List[Dict],
    train_ratio: float = 0.9,
    output_dir: str = "data",
) -> None:
    """Split data into train and validation sets."""
    if not data:
        print("No data to process")
        return
    
    split_idx = int(len(data) * train_ratio)
    train_data = data[:split_idx]
    val_data = data[split_idx:]
    
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    save_as_jsonl(train_data, str(output_dir / "train.jsonl"))
    save_as_jsonl(val_data, str(output_dir / "val.jsonl"))
    
    print(f"Saved {len(train_data)} training, {len(val_data)} validation samples")
    print(f"Train: {output_dir / 'train.jsonl'}")
    print(f"Val: {output_dir / 'val.jsonl'}")


def main():
    """Main entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(description="Prepare fine-tuning data")
    parser.add_argument(
        "--input", "-i",
        default="data/feedback.jsonl",
        help="Input feedback file",
    )
    parser.add_argument(
        "--output", "-o",
        default="data",
        help="Output directory",
    )
    parser.add_argument(
        "--train-ratio", "-r",
        type=float,
        default=0.9,
        help="Training split ratio",
    )
    
    args = parser.parse_args()
    
    # Load and filter positive feedback
    data = load_feedback(args.input)
    print(f"Loaded {len(data)} positive feedback records")
    
    # Split and save
    prepare_train_val_split(data, args.train_ratio, args.output)


if __name__ == "__main__":
    main()