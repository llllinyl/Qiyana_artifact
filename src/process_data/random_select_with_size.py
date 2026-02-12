import os
import random
import shutil
from pathlib import Path
import sys

def get_content_size(filepath):
    """Get content size in bytes starting from line 3"""
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if len(lines) >= 3:
            content = ''.join(lines[2:])
            return len(content.encode('utf-8'))
        else:
            return 0
    except Exception as e:
        print(f"  Failed to read {filepath}: {e}")
        return 0

def process_content(filepath, min_size_kb=10, max_size_kb=32):
    """
    Process document content based on size
    
    Returns:
        (success, processed_content, original_size_kb, process_type)
        process_type: 'keep', 'truncate', 'skip'
    """
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        if len(lines) < 3:
            return False, None, 0, 'skip'
        
        header = ''.join(lines[:2])
        content_lines = lines[2:]
        full_content = ''.join(content_lines)
        
        content_bytes = full_content.encode('utf-8')
        original_size = len(content_bytes)
        original_size_kb = original_size / 1024
        
        min_size_bytes = min_size_kb * 1024
        max_size_bytes = max_size_kb * 1024
        
        if original_size < min_size_bytes:
            return False, None, original_size_kb, 'skip'
        elif original_size <= max_size_bytes:
            return True, header + full_content, original_size_kb, 'keep'
        else:
            truncated_bytes = content_bytes[:max_size_bytes]
            
            while len(truncated_bytes) > 0 and truncated_bytes[-1] & 0b11000000 == 0b10000000:
                truncated_bytes = truncated_bytes[:-1]
            
            truncated_content = truncated_bytes.decode('utf-8', errors='ignore')
            
            return True, header + truncated_content, original_size_kb, 'truncate'
        
    except Exception as e:
        print(f"  Failed to process {filepath}: {e}")
        return False, None, 0, 'error'

def random_select_with_size_range(input_dir, output_dir, total_files=72172, select_count=16384, 
                                 min_size_kb=10, max_size_kb=32, start_index=0, seed=None):
    """
    Randomly select files and process based on size range
    """
    if seed is not None:
        random.seed(seed)
        print(f"Using random seed: {seed}")
    
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    end_index = start_index + total_files - 1
    
    print(f"File range: {start_index}.txt to {end_index}.txt")
    print(f"Total files: {total_files}")
    print(f"Files to select: {select_count}")
    print(f"Size requirement: >{min_size_kb}KB (skip if <{min_size_kb}KB)")
    print(f"Truncate size: {max_size_kb}KB (truncate if >{max_size_kb}KB)")
    print(f"Selection ratio: {select_count/total_files*100:.2f}%")
    
    all_indices = list(range(start_index, start_index + total_files))
    
    stats = {
        'total_available': total_files,
        'checked': 0,
        'qualified': 0,
        'kept': 0,
        'truncated': 0,
        'skipped_small': 0,
        'selected': 0,
        'copied': 0,
        'missing': 0,
        'errors': 0,
        'size_distribution': {
            '10-20KB': 0,
            '20-32KB': 0,
            '32-50KB': 0,
            '50-100KB': 0,
            '100KB+': 0
        }
    }
    
    print(f"\nScanning for qualified files (≥{min_size_kb}KB)...")
    qualified_files = []
    
    for idx in all_indices:
        filename = f"{idx}.txt"
        filepath = Path(input_dir) / filename
        
        stats['checked'] += 1
        
        if not filepath.exists():
            stats['missing'] += 1
            if stats['missing'] <= 3:
                print(f"  File {filename} does not exist")
            continue
        
        success, processed_content, original_size_kb, process_type = process_content(
            filepath, min_size_kb, max_size_kb
        )
        
        if success:
            if original_size_kb < 20:
                stats['size_distribution']['10-20KB'] += 1
            elif original_size_kb < 32:
                stats['size_distribution']['20-32KB'] += 1
            elif original_size_kb < 50:
                stats['size_distribution']['32-50KB'] += 1
            elif original_size_kb < 100:
                stats['size_distribution']['50-100KB'] += 1
            else:
                stats['size_distribution']['100KB+'] += 1
            
            qualified_files.append((idx, processed_content, original_size_kb, process_type))
            stats['qualified'] += 1
            
            if process_type == 'keep':
                stats['kept'] += 1
            elif process_type == 'truncate':
                stats['truncated'] += 1
            
            if stats['qualified'] % 1000 == 0:
                print(f"  Found {stats['qualified']} qualified files")
        else:
            if process_type == 'skip':
                stats['skipped_small'] += 1
            elif process_type == 'error':
                stats['errors'] += 1
    
    print(f"Scan completed!")
    print(f"Checked {stats['checked']} files")
    print(f"Qualified files (≥{min_size_kb}KB): {stats['qualified']}")
    print(f"  Kept as is (10-32KB): {stats['kept']}")
    print(f"  Need truncation (>32KB): {stats['truncated']}")
    print(f"  Skipped (<{min_size_kb}KB): {stats['skipped_small']}")
    print(f"  Missing files: {stats['missing']}")
    print(f"  Processing errors: {stats['errors']}")
    
    print(f"\nOriginal size distribution:")
    for range_name, count in stats['size_distribution'].items():
        if count > 0:
            percentage = count / stats['qualified'] * 100 if stats['qualified'] > 0 else 0
            print(f"  {range_name}: {count} ({percentage:.1f}%)")
    
    if len(qualified_files) < select_count:
        print(f"\n⚠️  Warning: Only {len(qualified_files)} files meet requirements,")
        print(f"        but {select_count} files are needed")
        print(f"        Will select all qualified files")
        select_count = min(select_count, len(qualified_files))
    
    print(f"\nRandomly selecting {select_count} from {len(qualified_files)} qualified files...")
    selected_files = random.sample(qualified_files, select_count)
    random.shuffle(selected_files)
    
    print(f"Randomly selected {len(selected_files)} files")
    
    selected_stats = {
        'keep': 0,
        'truncate': 0,
        'avg_original_size': 0,
        'min_original_size': float('inf'),
        'max_original_size': 0
    }
    
    if selected_files:
        original_sizes = [size for _, _, size, _ in selected_files]
        selected_stats['avg_original_size'] = sum(original_sizes) / len(original_sizes)
        selected_stats['min_original_size'] = min(original_sizes)
        selected_stats['max_original_size'] = max(original_sizes)
        
        for _, _, _, process_type in selected_files:
            selected_stats[process_type] += 1
    
    print(f"Selected files statistics:")
    print(f"  Kept as is: {selected_stats['keep']}")
    print(f"  Truncated: {selected_stats['truncate']}")
    print(f"  Original size: {selected_stats['min_original_size']:.1f}KB - {selected_stats['max_original_size']:.1f}KB")
    print(f"  Average size: {selected_stats['avg_original_size']:.1f}KB")
    
    print(f"\nSaving files to output directory...")
    for new_index, (old_index, processed_content, original_size_kb, process_type) in enumerate(selected_files):
        new_filename = f"{new_index}.txt"
        new_path = output_path / new_filename
        
        try:
            with open(new_path, 'w', encoding='utf-8') as f:
                f.write(processed_content)
            
            stats['copied'] += 1
            
            if stats['copied'] % 1000 == 0:
                print(f"  Saved {stats['copied']}/{select_count} files")
                
        except Exception as e:
            stats['errors'] += 1
            print(f"  Error: Failed to save {new_filename}: {e}")
    
    create_mapping_file(output_dir, selected_files, start_index, min_size_kb, max_size_kb)
    create_stats_file(output_dir, stats, selected_stats, select_count, seed, min_size_kb, max_size_kb)
    
    print(f"\n{'='*60}")
    print(f"Processing completed!")
    print(f"{'='*60}")
    print(f"Input directory: {input_dir}")
    print(f"Output directory: {output_dir}")
    print(f"Total files: {stats['total_available']}")
    print(f"Files checked: {stats['checked']}")
    print(f"Qualified files (≥{min_size_kb}KB): {stats['qualified']} ({stats['qualified']/stats['checked']*100:.1f}%)")
    print(f"  Kept as is ({min_size_kb}-{max_size_kb}KB): {stats['kept']}")
    print(f"  Truncated (>{max_size_kb}KB): {stats['truncated']}")
    print(f"  Skipped (<{min_size_kb}KB): {stats['skipped_small']}")
    print(f"Successfully saved: {stats['copied']} files")
    
    if stats['copied'] < select_count:
        print(f"\n⚠️  Warning: Only saved {stats['copied']} files, less than target {select_count}")
    
    print(f"\nNew files: 0.txt to {stats['copied']-1}.txt")
    print(f"Processing: Files >{max_size_kb}KB truncated to {max_size_kb}KB, {min_size_kb}-{max_size_kb}KB kept as is")
    print(f"Mapping file: {output_dir}/mapping.txt")
    print(f"Stats file: {output_dir}/stats.txt")

def create_mapping_file(output_dir, selected_files, start_index, min_size_kb, max_size_kb):
    """Create mapping file"""
    mapping_file = os.path.join(output_dir, "mapping.txt")
    
    try:
        with open(mapping_file, 'w', encoding='utf-8') as f:
            f.write("File Mapping Table\n")
            f.write("=" * 70 + "\n")
            f.write(f"Size requirement: >{min_size_kb}KB (skip if <{min_size_kb}KB)\n")
            f.write(f"Truncate size: {max_size_kb}KB (truncate if >{max_size_kb}KB)\n")
            f.write(f"Total files: {len(selected_files)}\n")
            f.write(f"Start index: {start_index}\n")
            f.write("\n")
            
            f.write("No. | New File | Original File | Original Size | Processing\n")
            f.write("-" * 70 + "\n")
            
            for new_index, (old_index, _, original_size_kb, process_type) in enumerate(selected_files[:50]):
                process_desc = "Kept" if process_type == 'keep' else "Truncated"
                f.write(f"{new_index:4d} | {new_index:6d}.txt -> {old_index:6d}.txt | {original_size_kb:7.1f}KB | {process_desc}\n")
            
            if len(selected_files) > 50:
                f.write(f"... and {len(selected_files) - 50} more files\n")
            
            f.write("\n" + "=" * 70 + "\n")
            f.write("Selected Files Processing Statistics:\n")
            
            keep_count = sum(1 for _, _, _, pt in selected_files if pt == 'keep')
            truncate_count = sum(1 for _, _, _, pt in selected_files if pt == 'truncate')
            
            if selected_files:
                original_sizes = [size for _, _, size, _ in selected_files]
                avg_size = sum(original_sizes) / len(original_sizes)
                min_size = min(original_sizes)
                max_size = max(original_sizes)
                
                f.write(f"  Kept as is: {keep_count} ({keep_count/len(selected_files)*100:.1f}%)\n")
                f.write(f"  Truncated: {truncate_count} ({truncate_count/len(selected_files)*100:.1f}%)\n")
                f.write(f"  Original size range: {min_size:.1f}KB - {max_size:.1f}KB\n")
                f.write(f"  Average original size: {avg_size:.1f}KB\n")
        
        print(f"Mapping file created: {mapping_file}")
    except Exception as e:
        print(f"Failed to create mapping file: {e}")

def create_stats_file(output_dir, stats, selected_stats, select_count, seed, min_size_kb, max_size_kb):
    """Create statistics file"""
    stats_file = os.path.join(output_dir, "stats.txt")
    
    try:
        with open(stats_file, 'w', encoding='utf-8') as f:
            f.write("Random Selection and Processing Statistics\n")
            f.write("=" * 60 + "\n\n")
            
            f.write("[Processing Rules]\n")
            f.write(f"  Minimum requirement: >{min_size_kb}KB (skip if smaller)\n")
            f.write(f"  Truncate size: {max_size_kb}KB (truncate if larger)\n")
            f.write(f"  Keep range: {min_size_kb}-{max_size_kb}KB (keep as is)\n\n")
            
            f.write("[File Statistics]\n")
            f.write(f"  Total available files: {stats['total_available']}\n")
            f.write(f"  Target selection count: {select_count}\n")
            f.write(f"  Files checked: {stats['checked']}\n")
            f.write(f"  Qualified files (≥{min_size_kb}KB): {stats['qualified']} ({stats['qualified']/stats['checked']*100:.1f}%)\n")
            f.write(f"    Kept as is ({min_size_kb}-{max_size_kb}KB): {stats['kept']}\n")
            f.write(f"    Need truncation (>{max_size_kb}KB): {stats['truncated']}\n")
            f.write(f"    Skipped (<{min_size_kb}KB): {stats['skipped_small']}\n")
            f.write(f"    Missing files: {stats['missing']}\n")
            f.write(f"    Processing errors: {stats['errors']}\n")
            f.write(f"  Successfully saved: {stats['copied']}\n\n")
            
            f.write("[Original Size Distribution]\n")
            for range_name, count in stats['size_distribution'].items():
                if count > 0:
                    percentage = count / stats['qualified'] * 100 if stats['qualified'] > 0 else 0
                    f.write(f"  {range_name}: {count} ({percentage:.1f}%)\n")
            
            f.write(f"\n[Selected Files Statistics]\n")
            f.write(f"  Kept as is: {selected_stats['keep']} ({selected_stats['keep']/stats['copied']*100:.1f}%)\n")
            f.write(f"  Truncated: {selected_stats['truncate']} ({selected_stats['truncate']/stats['copied']*100:.1f}%)\n")
            f.write(f"  Minimum original size: {selected_stats['min_original_size']:.1f}KB\n")
            f.write(f"  Maximum original size: {selected_stats['max_original_size']:.1f}KB\n")
            f.write(f"  Average original size: {selected_stats['avg_original_size']:.1f}KB\n\n")
            
            if seed is not None:
                f.write(f"Random seed: {seed}\n\n")
            
            if stats['copied'] < select_count:
                f.write(f"⚠️  Warning: Only saved {stats['copied']} files, less than target {select_count}\n")
                f.write(f"   Possible cause: Insufficient qualified files\n\n")
            
            f.write("[Output Files]\n")
            f.write(f"  File range: 0.txt to {stats['copied']-1}.txt\n")
            f.write(f"  Final size: ≤{max_size_kb}KB\n")
            f.write(f"  Mapping file: mapping.txt\n")
            f.write(f"  Stats file: stats.txt\n")
        
        print(f"Statistics file created: {stats_file}")
    except Exception as e:
        print(f"Failed to create statistics file: {e}")

def verify_output(output_dir, expected_count=16384, max_size_kb=32):
    """Verify output files"""
    print(f"\n{'='*60}")
    print(f"Verifying output files...")
    
    output_path = Path(output_dir)
    
    output_files = list(output_path.glob("*.txt"))
    output_files = [f for f in output_files if f.name not in ['mapping.txt', 'stats.txt']]
    
    print(f"Files in output directory: {len(output_files)}")
    print(f"Expected file count: {expected_count}")
    
    if len(output_files) == expected_count:
        print("✅ File count correct")
    else:
        print(f"❌ File count incorrect: expected {expected_count}, got {len(output_files)}")
    
    indices = []
    for file in output_files:
        try:
            idx = int(file.stem)
            indices.append(idx)
        except:
            continue
    
    indices.sort()
    
    if indices == list(range(len(indices))):
        print("✅ File indices are continuous")
    else:
        print("❌ File indices are not continuous")
    
    print(f"\nChecking content size (from line 3, should be ≤{max_size_kb}KB):")
    
    size_stats = {
        'total': 0,
        'correct': 0,
        'too_large': 0,
        'too_small': 0,
        'errors': 0,
        'sizes': []
    }
    
    max_size_bytes = max_size_kb * 1024
    
    sample_count = min(100, len(output_files))
    sample_files = random.sample(output_files, sample_count) if output_files else []
    
    for file in sample_files:
        try:
            with open(file, 'r', encoding='utf-8') as f:
                lines = f.readlines()
            
            if len(lines) >= 3:
                content = ''.join(lines[2:])
                content_size = len(content.encode('utf-8'))
                content_size_kb = content_size / 1024
                
                size_stats['total'] += 1
                size_stats['sizes'].append(content_size_kb)
                
                if content_size <= max_size_bytes:
                    size_stats['correct'] += 1
                else:
                    size_stats['too_large'] += 1
                    print(f"  ❌ {file.name}: {content_size_kb:.1f}KB > {max_size_kb}KB")
            else:
                size_stats['errors'] += 1
                
        except Exception as e:
            size_stats['errors'] += 1
            print(f"  ❌ {file.name}: Failed to read - {e}")
    
    if size_stats['total'] > 0:
        correct_ratio = size_stats['correct'] / size_stats['total'] * 100
        
        print(f"\nSampling results ({size_stats['total']} files):")
        print(f"  Size correct (≤{max_size_kb}KB): {size_stats['correct']} ({correct_ratio:.1f}%)")
        print(f"  Too large (>{max_size_kb}KB): {size_stats['too_large']}")
        print(f"  Read errors: {size_stats['errors']}")
        
        if size_stats['sizes']:
            avg_size = sum(size_stats['sizes']) / len(size_stats['sizes'])
            min_size = min(size_stats['sizes'])
            max_size = max(size_stats['sizes'])
            print(f"  Size range: {min_size:.1f}KB - {max_size:.1f}KB")
            print(f"  Average size: {avg_size:.1f}KB")
    
    total_size_bytes = sum(f.stat().st_size for f in output_files)
    avg_size_bytes = total_size_bytes / len(output_files) if output_files else 0
    
    print(f"\nOverall statistics:")
    print(f"  Total size: {total_size_bytes/1024/1024:.2f} MB")
    print(f"  Average file size: {avg_size_bytes/1024:.2f} KB")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Randomly select files and process based on size range')
    parser.add_argument('input_dir', help='Input directory (contains 0.txt, 1.txt, ...)')
    parser.add_argument('output_dir', help='Output directory')
    parser.add_argument('--total-files', type=int, default=72172,
                       help='Total number of files (default: 72172)')
    parser.add_argument('--select-count', type=int, default=16384,
                       help='Number of files to select (default: 16384)')
    parser.add_argument('--min-size', type=int, default=10,
                       help='Minimum content size in KB (default: 10) - skip if smaller')
    parser.add_argument('--max-size', type=int, default=32,
                       help='Maximum content size in KB (default: 32) - truncate if larger')
    parser.add_argument('--start-index', type=int, default=0,
                       help='Start index (default: 0)')
    parser.add_argument('--seed', type=int, default=None,
                       help='Random seed')
    parser.add_argument('--verify', action='store_true',
                       help='Verify results after processing')
    
    args = parser.parse_args()
    
    if args.select_count > args.total_files:
        print(f"Error: Selection count ({args.select_count}) cannot exceed total files ({args.total_files})")
        sys.exit(1)
    
    if args.min_size <= 0:
        print(f"Error: Minimum size must be greater than 0")
        sys.exit(1)
    
    if args.max_size <= args.min_size:
        print(f"Error: Maximum size ({args.max_size}KB) must be greater than minimum size ({args.min_size}KB)")
        sys.exit(1)
    
    random_select_with_size_range(
        args.input_dir, 
        args.output_dir, 
        args.total_files, 
        args.select_count, 
        args.min_size, 
        args.max_size, 
        args.start_index, 
        args.seed
    )
    
    if args.verify:
        verify_output(args.output_dir, args.select_count, args.max_size)

if __name__ == '__main__':
    main()
