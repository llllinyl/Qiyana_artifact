import os
import re
import sys
from pathlib import Path

def is_content_only_title(title, content):
    if not title or not content:
        return False
    
    cleaned_content = re.sub(r'\s+', ' ', content.strip())
    cleaned_title = re.sub(r'\s+', ' ', title.strip())
    
    if not cleaned_content:
        return True
    
    if cleaned_content.lower() == cleaned_title.lower():
        return True
    
    content_lower = cleaned_content.lower()
    title_lower = cleaned_title.lower()
    
    if content_lower.startswith(title_lower):
        remaining = content_lower[len(title_lower):].strip()
        if not remaining or re.match(r'^[^\w\s]*$', remaining):
            return True
    
    title_words = cleaned_title.lower().split()
    content_words = cleaned_content.lower().split()
    
    if not title_words:
        return False
    
    if len(content_words) <= 3:
        if all(word in title_words for word in content_words):
            return True
    
    if len(content_words) > 0:
        title_word_set = set(title_words)
        common_words = sum(1 for word in content_words if word in title_word_set)
        if common_words / len(content_words) > 0.8 and len(content_words) < 5:
            return True
    
    return False

def remove_title_from_content(title, content):
    if not title or not content:
        return content
    
    content_stripped = content.strip()
    title_stripped = title.strip()
    
    if not content_stripped or not title_stripped:
        return content
    
    content_lower = content_stripped.lower()
    title_lower = title_stripped.lower()
    
    if content_lower.startswith(title_lower):
        remaining = content_stripped[len(title_stripped):]
        
        if remaining and remaining[0] in '.,;:!?\n\t ':
            return remaining.lstrip('.,;:!?\n\t ')
        else:
            return content_stripped
    
    return content_stripped

def get_content_preview(content, max_bytes=250, single_line=True):
    if not content:
        return ""
    
    if single_line:
        content = re.sub(r'\s+', ' ', content)
    
    content_bytes = content.encode('utf-8')
    
    if len(content_bytes) <= max_bytes:
        return content
    
    truncated_bytes = content_bytes[:max_bytes]
    
    while len(truncated_bytes) > 0 and truncated_bytes[-1] & 0b11000000 == 0b10000000:
        truncated_bytes = truncated_bytes[:-1]
    
    result = truncated_bytes.decode('utf-8', errors='ignore')
    
    if single_line:
        result = result.replace('\n', ' ').replace('\r', ' ')
    
    return result

def extract_and_save_docs(input_dir, output_dir, preview_bytes=250, start_index=0, remove_leading_title=True, single_line_preview=True):
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    stats = {
        'total_docs': 0,
        'empty_docs': 0,
        'title_only_docs': 0,
        'title_removed_docs': 0,
        'valid_docs': 0,
        'doc_index': start_index
    }
    
    input_path = Path(input_dir)
    
    wiki_files = list(input_path.rglob("wiki_*"))
    if not wiki_files:
        wiki_files = list(input_path.rglob("wiki_??"))
    
    print(f"Found {len(wiki_files)} wiki files")
    print(f"Files will be numbered starting from index {start_index}")
    print(f"Preview size: {preview_bytes} bytes")
    print(f"Remove leading title from content: {'Yes' if remove_leading_title else 'No'}")
    print(f"Single-line preview: {'Yes' if single_line_preview else 'No'}")
    
    doc_pattern = re.compile(r'<doc\s+([^>]+)>(.*?)</doc>', re.DOTALL)
    
    for wiki_file in sorted(wiki_files):
        print(f"Processing file: {wiki_file}")
        
        try:
            with open(wiki_file, 'r', encoding='utf-8', errors='ignore') as f:
                content = f.read()
        except Exception as e:
            print(f"  Failed to read file: {e}")
            continue
        
        matches = list(doc_pattern.finditer(content))
        print(f"  Found {len(matches)} documents")
        
        for match in matches:
            stats['total_docs'] += 1
            attrs_str = match.group(1)
            doc_content = match.group(2)
            doc_content_stripped = doc_content.strip()
            
            if not doc_content_stripped or doc_content_stripped.isspace():
                stats['empty_docs'] += 1
                continue
            
            attrs = {}
            attr_matches = re.findall(r'(\w+)="([^"]*)"', attrs_str)
            for key, value in attr_matches:
                attrs[key] = value
            
            title = attrs.get('title', 'Untitled')
            
            if is_content_only_title(title, doc_content_stripped):
                stats['title_only_docs'] += 1
                continue
            
            if 'id' not in attrs:
                attrs['id'] = str(stats['total_docs'])
            if 'title' not in attrs:
                attrs['title'] = f"Document_{stats['doc_index']}"
            if 'url' not in attrs:
                attrs['url'] = f"https://example.com/doc/{attrs['id']}"
            
            final_content = doc_content_stripped
            if remove_leading_title:
                content_before = final_content
                final_content = remove_title_from_content(title, final_content)
                if final_content != content_before:
                    stats['title_removed_docs'] += 1
            
            output_file = output_path / f"{stats['doc_index']}.txt"
            
            content_preview = get_content_preview(final_content, preview_bytes, single_line_preview)
            
            try:
                with open(output_file, 'w', encoding='utf-8') as f:
                    f.write(attrs['title'] + '\n')
                    f.write(content_preview + '\n')
                    f.write(final_content + '\n')
                
                stats['valid_docs'] += 1
                
                if stats['valid_docs'] % 100 == 0:
                    print(f"  Saved {stats['valid_docs']} valid documents (index: {stats['doc_index']})")
                    preview_display = content_preview
                    if len(preview_display) > 60:
                        preview_display = preview_display[:57] + "..."
                    print(f"    Preview example: {preview_display}")
                
                stats['doc_index'] += 1
                    
            except Exception as e:
                print(f"  Failed to write file {output_file}: {e}")
    
    print(f"\n{'='*50}")
    print(f"Processing completed!")
    print(f"{'='*50}")
    print(f"Total documents found: {stats['total_docs']}")
    print(f"Empty documents: {stats['empty_docs']}")
    print(f"Title-only documents: {stats['title_only_docs']}")
    if remove_leading_title:
        print(f"Documents with title removed: {stats['title_removed_docs']}")
    print(f"Valid documents: {stats['valid_docs']}")
    print(f"Filter rate: {(stats['empty_docs'] + stats['title_only_docs']) / stats['total_docs'] * 100:.1f}%")
    print(f"File index range: {start_index} to {stats['doc_index']-1}")
    print(f"Preview size: {preview_bytes} bytes")
    print(f"Single-line preview: {'Yes' if single_line_preview else 'No'}")
    print(f"Saved to: {output_dir}/")
    print(f"File naming: {start_index}.txt, {start_index+1}.txt, ..., {stats['doc_index']-1}.txt")
    
    create_stats_file(output_dir, stats, start_index, preview_bytes, remove_leading_title, single_line_preview)
    create_index_file(output_dir, stats['valid_docs'], start_index, preview_bytes, single_line_preview)

def create_stats_file(output_dir, stats, start_index, preview_bytes, remove_leading_title, single_line_preview):
    stats_file = os.path.join(output_dir, "stats.txt")
    
    try:
        with open(stats_file, 'w', encoding='utf-8') as f:
            f.write("Document Processing Statistics\n")
            f.write("=" * 40 + "\n\n")
            
            f.write(f"Total documents: {stats['total_docs']}\n")
            f.write(f"Empty documents: {stats['empty_docs']}\n")
            f.write(f"Title-only documents: {stats['title_only_docs']}\n")
            if remove_leading_title:
                f.write(f"Documents with title removed: {stats['title_removed_docs']}\n")
            f.write(f"Valid documents: {stats['valid_docs']}\n")
            f.write(f"Start index: {start_index}\n")
            f.write(f"End index: {stats['doc_index']-1}\n")
            f.write(f"Preview bytes: {preview_bytes}\n")
            f.write(f"Remove leading title: {'Yes' if remove_leading_title else 'No'}\n")
            f.write(f"Single-line preview: {'Yes' if single_line_preview else 'No'}\n\n")
            
            f.write(f"Filter rate: {(stats['empty_docs'] + stats['title_only_docs']) / stats['total_docs'] * 100:.1f}%\n")
            f.write(f"Save rate: {stats['valid_docs'] / stats['total_docs'] * 100:.1f}%\n\n")
            
            f.write("File format:\n")
            f.write("  Line 1: Document title\n")
            f.write(f"  Line 2: First {preview_bytes} bytes of document content (preview)")
            if single_line_preview:
                f.write(" - single line, newlines replaced with spaces")
            f.write("\n")
            f.write("  Line 3+: Full document content\n\n")
            
            f.write("Filtering criteria:\n")
            f.write("1. Document content is empty or only whitespace\n")
            f.write("2. Document content is just the title itself\n")
            f.write("3. Document content is just the title plus spaces/newlines\n")
            f.write("4. Document content is just the title plus punctuation\n")
            f.write("5. Document content is mainly simple repetition of the title\n")
            if remove_leading_title:
                f.write("\nNote: Leading title has been removed from content to avoid duplication\n")
        
        print(f"Statistics file created: {stats_file}")
    except Exception as e:
        print(f"Failed to create statistics file: {e}")

def create_index_file(output_dir, total_docs, start_index, preview_bytes, single_line_preview):
    index_file = os.path.join(output_dir, "index.txt")
    
    try:
        with open(index_file, 'w', encoding='utf-8') as f:
            f.write("Document Index\n")
            f.write("=" * 40 + "\n")
            f.write(f"Total valid documents: {total_docs}\n")
            f.write(f"Start index: {start_index}\n")
            f.write(f"End index: {start_index + total_docs - 1}\n")
            f.write(f"Preview bytes: {preview_bytes}\n")
            if single_line_preview:
                f.write("Preview format: Single line (newlines replaced with spaces)\n")
            f.write("File naming format: [index].txt\n")
            f.write("Each file contains:\n")
            f.write(f"  Line 1: Document title\n")
            f.write(f"  Line 2: First {preview_bytes} bytes of document content (preview)")
            if single_line_preview:
                f.write(" - single line")
            f.write("\n")
            f.write("  Line 3+: Full document content\n\n")
            f.write("File list:\n")
            
            max_show = min(total_docs, 20)
            for i in range(max_show):
                file_index = start_index + i
                filename = f"{file_index}.txt"
                filepath = os.path.join(output_dir, filename)
                if os.path.exists(filepath):
                    try:
                        with open(filepath, 'r', encoding='utf-8') as doc_file:
                            title = doc_file.readline().strip()
                            preview = doc_file.readline().strip()
                        preview_display = preview
                        if len(preview_display) > 50:
                            preview_display = preview_display[:47] + "..."
                        f.write(f"{file_index:4d}.txt -> {title}\n")
                        f.write(f"       Preview: {preview_display}\n")
                    except:
                        f.write(f"{file_index:4d}.txt -> [Read failed]\n")
            
            if total_docs > 20:
                f.write(f"\n... and {total_docs - 20} more documents\n")
    
        print(f"Index file created: {index_file}")
    except Exception as e:
        print(f"Failed to create index file: {e}")

def process_single_file(input_file, output_dir, preview_bytes=250, start_index=0, remove_leading_title=True, single_line_preview=True):
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    stats = {
        'total_docs': 0,
        'empty_docs': 0,
        'title_only_docs': 0,
        'title_removed_docs': 0,
        'valid_docs': 0,
        'doc_index': start_index
    }
    
    print(f"Processing file: {input_file}")
    print(f"Files will be numbered starting from index {start_index}")
    print(f"Preview size: {preview_bytes} bytes")
    print(f"Remove leading title from content: {'Yes' if remove_leading_title else 'No'}")
    print(f"Single-line preview: {'Yes' if single_line_preview else 'No'}")
    
    try:
        with open(input_file, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()
    except Exception as e:
        print(f"Failed to read file: {e}")
        return
    
    doc_pattern = re.compile(r'<doc\s+([^>]+)>(.*?)</doc>', re.DOTALL)
    
    matches = list(doc_pattern.finditer(content))
    print(f"Found {len(matches)} documents")
    
    for match in matches:
        stats['total_docs'] += 1
        attrs_str = match.group(1)
        doc_content = match.group(2)
        doc_content_stripped = doc_content.strip()
        
        if not doc_content_stripped or doc_content_stripped.isspace():
            stats['empty_docs'] += 1
            continue
        
        attrs = {}
        attr_matches = re.findall(r'(\w+)="([^"]*)"', attrs_str)
        for key, value in attr_matches:
            attrs[key] = value
        
        title = attrs.get('title', 'Untitled')
        
        if is_content_only_title(title, doc_content_stripped):
            stats['title_only_docs'] += 1
            continue
        
        if 'id' not in attrs:
            attrs['id'] = str(stats['total_docs'])
        if 'title' not in attrs:
            attrs['title'] = f"Document_{stats['doc_index']}"
        if 'url' not in attrs:
            attrs['url'] = f"https://example.com/doc/{attrs['id']}"
        
        final_content = doc_content_stripped
        if remove_leading_title:
            content_before = final_content
            final_content = remove_title_from_content(title, final_content)
            if final_content != content_before:
                stats['title_removed_docs'] += 1
        
        output_file = output_path / f"{stats['doc_index']}.txt"
        
        content_preview = get_content_preview(final_content, preview_bytes, single_line_preview)
        
        try:
            with open(output_file, 'w', encoding='utf-8') as f:
                f.write(attrs['title'] + '\n')
                f.write(content_preview + '\n')
                f.write(final_content + '\n')
            
            stats['valid_docs'] += 1
            
            if stats['valid_docs'] % 100 == 0:
                print(f"  Saved {stats['valid_docs']} valid documents (index: {stats['doc_index']})")
            
            stats['doc_index'] += 1
                
        except Exception as e:
            print(f"  Failed to write file {output_file}: {e}")
    
    print(f"\n{'='*50}")
    print(f"Processing completed!")
    print(f"{'='*50}")
    print(f"Total documents found: {stats['total_docs']}")
    print(f"Empty documents: {stats['empty_docs']}")
    print(f"Title-only documents: {stats['title_only_docs']}")
    if remove_leading_title:
        print(f"Documents with title removed: {stats['title_removed_docs']}")
    print(f"Valid documents: {stats['valid_docs']}")
    print(f"File index range: {start_index} to {stats['doc_index']-1}")
    print(f"Saved to: {output_dir}/")
    
    create_stats_file(output_dir, stats, start_index, preview_bytes, remove_leading_title, single_line_preview)
    create_index_file(output_dir, stats['valid_docs'], start_index, preview_bytes, single_line_preview)

def show_example():
    print("Before and After Processing Example:")
    print("=" * 70)
    print("Original content:")
    print('''<doc id="12" url="https://en.wikipedia.org/wiki/Anarchism" title="Anarchism">
Anarchism
Anarchism is a political philosophy
and movement that is skeptical...
</doc>''')
    print()
    print("Processed file content (34.txt):")
    print("Line 1: Anarchism")
    print("Line 2: is a political philosophy and movement that is skeptical... (first 250 bytes, single line)")
    print("Line 3+: is a political philosophy\nand movement that is skeptical...")
    print("=" * 70)
    print("Note:")
    print("1. Leading title 'Anarchism' has been removed from content")
    print("2. Line 2 (preview) has newlines replaced with spaces, kept as single line")
    print("3. Full content from Line 3+ preserves original formatting")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='Extract non-empty wiki documents and format output')
    parser.add_argument('input', help='Input file or directory')
    parser.add_argument('output_dir', help='Output directory')
    parser.add_argument('--preview-bytes', type=int, default=250,
                       help='Maximum bytes for preview text (default: 250)')
    parser.add_argument('--start-index', type=int, default=0,
                       help='Starting index for files (default: 0)')
    parser.add_argument('--single-file', action='store_true',
                       help='Process single file instead of directory')
    parser.add_argument('--keep-title', action='store_true',
                       help='Keep leading title in content (default: remove)')
    parser.add_argument('--multiline-preview', action='store_true',
                       help='Keep multiline format for preview (default: single line)')
    parser.add_argument('--example', action='store_true',
                       help='Show processing example')
    
    args = parser.parse_args()
    
    if args.example:
        show_example()
        return
    
    remove_leading_title = not args.keep_title
    single_line_preview = not args.multiline_preview
    
    if args.single_file:
        process_single_file(args.input, args.output_dir, args.preview_bytes, 
                          args.start_index, remove_leading_title, single_line_preview)
    else:
        extract_and_save_docs(args.input, args.output_dir, args.preview_bytes,
                            args.start_index, remove_leading_title, single_line_preview)

if __name__ == '__main__':
    main()
