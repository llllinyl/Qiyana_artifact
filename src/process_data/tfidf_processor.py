import os
import re
import math
from collections import defaultdict, Counter
import numpy as np
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing as mp


def process_chunk_task(db_folder, file_names, keywords_set, chunk_start_idx, doc_freq, num_docs):
    results = []
    chunk_word_counts = []

    for local_idx, file_name in enumerate(file_names):
        file_path = os.path.join(db_folder, file_name)
        doc_idx = chunk_start_idx + local_idx

        try:
            with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.readlines()

            word_counts = Counter()

            if len(lines) >= 3:
                content = " ".join(lines[2:]).lower()
                content = re.sub(r"[^\w\s]", " ", content)
                words = content.split()

                stop_words = {
                    "a", "an", "the", "and", "or", "but", "in", "on", "at",
                    "to", "for", "of", "with", "by"
                }

                for word in words:
                    if len(word) > 1 and word not in stop_words and word in keywords_set:
                        word_counts[word] += 1

            chunk_word_counts.append((doc_idx, word_counts))

            total_words = sum(word_counts.values())
            if total_words > 0 and doc_freq is not None:
                tfidf_vector = {}
                for word, count in word_counts.items():
                    if word in doc_freq:
                        tf = count / total_words
                        idf = math.log((1 + num_docs) / (1 + doc_freq[word])) + 1.0
                        tfidf_vector[word] = tf * idf
                results.append((doc_idx, tfidf_vector))
            else:
                results.append((doc_idx, {}))

        except Exception as e:
            print(f"Error processing {file_name}: {e}")
            chunk_word_counts.append((doc_idx, Counter()))
            results.append((doc_idx, {}))

    return chunk_word_counts, results


class TFIDFProcessor:
    def __init__(
        self,
        db_folder,
        keyword_file,
        output_file,
        max_words=65536,
        num_bins=1024,
        chunk_size=5000,
        max_docs=16384
    ):
        self.db_folder = db_folder
        self.keyword_file = keyword_file
        self.output_file = output_file
        self.max_words = max_words
        self.num_bins = num_bins
        self.chunk_size = chunk_size
        self.max_docs = max_docs

        all_txt_files = [
            f for f in os.listdir(db_folder)
            if f.endswith(".txt") and f[:-4].isdigit()
        ]
        all_txt_files = sorted(all_txt_files, key=lambda x: int(x[:-4]))

        self.txt_files = [
            f for f in all_txt_files
            if 0 <= int(f[:-4]) < self.max_docs
        ]

        self.txt_files = self.txt_files[:self.max_docs]
        self.num_docs = len(self.txt_files)

        self.all_keywords = []
        self.word_to_id = {}
        self.id_to_word = {}
        self.doc_freq = {}
        self.tfidf_vectors = [None] * self.num_docs

    def load_keywords(self):
        print(f"Loading keywords from {self.keyword_file}...")

        raw_keywords = []

        with open(self.keyword_file, "r", encoding="utf-8") as f:
            for i, line in enumerate(f):
                if i >= self.max_docs:
                    break

                line = line.strip()
                if not line:
                    continue

                raw_keywords.extend(
                    kw.strip().lower()
                    for kw in line.split(",")
                    if kw.strip()
                )

        self.all_keywords = list(dict.fromkeys(raw_keywords))

        print(f"Loaded {self.max_docs} lines from keyword file")
        print(f"Total keywords before deduplication: {len(raw_keywords)}")
        print(f"Unique keywords after deduplication: {len(self.all_keywords)}")

        return self.all_keywords

    def build_vocabulary(self, num_workers=None):
        if num_workers is None:
            num_workers = mp.cpu_count()

        self.load_keywords()
        keywords_set = set(self.all_keywords)
        print(f"Keyword set size: {len(keywords_set)}")

        if self.num_docs == 0:
            raise ValueError("No valid documents found in the specified range.")

        print(f"Using {self.num_docs} documents: {self.txt_files[0]} to {self.txt_files[-1]}")

        chunks = []
        for i in range(0, self.num_docs, self.chunk_size):
            chunk_files = self.txt_files[i:i + self.chunk_size]
            chunks.append((self.db_folder, chunk_files, keywords_set, i, None, self.num_docs))

        print(f"Processing {self.num_docs} documents in {len(chunks)} chunks with {num_workers} processes...")

        word_doc_counts = defaultdict(int)
        processed = 0

        with ProcessPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(process_chunk_task, *chunk) for chunk in chunks]

            for future in as_completed(futures):
                chunk_word_counts, _ = future.result()

                for _, word_counts in chunk_word_counts:
                    for word in set(word_counts.keys()):
                        word_doc_counts[word] += 1

                    processed += 1
                    if processed % 1000 == 0:
                        print(f"Processed {processed}/{self.num_docs} documents")

        print(f"Found {len(word_doc_counts)} keywords appearing in the documents")

        idf_scores = {}
        for word in self.all_keywords:
            doc_count = word_doc_counts.get(word, 0)
            idf_scores[word] = math.log((1 + self.num_docs) / (1 + doc_count)) + 1.0

        ranked_words = sorted(idf_scores.items(), key=lambda x: (-x[1], x[0]))
        top_word_set = set(word for word, _ in ranked_words[:self.max_words])

        selected_words = []
        for word in self.all_keywords:
            if word in top_word_set:
                selected_words.append((word, idf_scores[word]))

        self.word_to_id.clear()
        self.id_to_word.clear()

        for idx, (word, _) in enumerate(selected_words):
            self.word_to_id[word] = idx
            self.id_to_word[idx] = word

        self.doc_freq = {
            word: word_doc_counts.get(word, 0)
            for word in self.word_to_id.keys()
        }

        print(f"Selected {len(self.word_to_id)} keywords")

        idf_values = [idf_scores[w] for w in self.word_to_id.keys()]
        if idf_values:
            print(f"IDF range: min={min(idf_values):.6f}, max={max(idf_values):.6f}")

        return selected_words

    def calculate_tfidf_vectors(self, num_workers=None):
        if num_workers is None:
            num_workers = mp.cpu_count()

        print(f"Calculating TF-IDF vectors with {num_workers} processes...")

        chunks = []
        selected_keywords = set(self.word_to_id.keys())

        for i in range(0, self.num_docs, self.chunk_size):
            chunk_files = self.txt_files[i:i + self.chunk_size]
            chunks.append((self.db_folder, chunk_files, selected_keywords, i, self.doc_freq, self.num_docs))

        processed = 0

        with ProcessPoolExecutor(max_workers=num_workers) as executor:
            futures = [executor.submit(process_chunk_task, *chunk) for chunk in chunks]

            for future in as_completed(futures):
                _, chunk_tfidf = future.result()

                for doc_idx, tfidf_vector in chunk_tfidf:
                    self.tfidf_vectors[doc_idx] = tfidf_vector
                    processed += 1
                    if processed % 1000 == 0:
                        print(f"Calculated {processed}/{self.num_docs} documents")

        print("TF-IDF calculation completed")

    def quantize_and_save(self, top_words):
        print("Quantizing and saving results...")

        num_words = len(self.word_to_id)
        global_max = 0.0

        print("Scanning global TF-IDF maximum...")

        for i in range(self.num_docs):
            tfidf_vector = self.tfidf_vectors[i]
            if tfidf_vector:
                local_max = max(tfidf_vector.values(), default=0.0)
                if local_max > global_max:
                    global_max = local_max

            if (i + 1) % 1000 == 0:
                print(f"Scanned {i + 1}/{self.num_docs} documents")

        print(f"TF-IDF value range for quantization: 0.000000 to {global_max:.6f}")
        print(f"Quantization target range: 0 to {self.num_bins - 1}")
        print("Writing output file...")

        with open(self.output_file, "w", encoding="utf-8") as f:
            words = [word for word, _ in top_words]
            f.write(",".join(words) + "\n")

            for i in range(self.num_docs):
                row = np.zeros(num_words, dtype=np.float32)
                tfidf_vector = self.tfidf_vectors[i]

                for word, value in tfidf_vector.items():
                    if word in self.word_to_id:
                        row[self.word_to_id[word]] = value

                if global_max <= 1e-12:
                    quantized_row = np.zeros(num_words, dtype=np.int16)
                else:
                    normalized = row / global_max
                    normalized = np.clip(normalized, 0.0, 1.0)
                    quantized_row = np.rint(normalized * (self.num_bins - 1)).astype(np.int16)

                f.write(",".join(map(str, quantized_row.tolist())) + "\n")

                if (i + 1) % 1000 == 0:
                    print(f"Saved {i + 1}/{self.num_docs} rows")
                    f.flush()

        file_size = os.path.getsize(self.output_file) / (1024 * 1024)
        print(f"Output file size: {file_size:.2f} MB")

    def process(self):
        print("=" * 60)
        print("TF-IDF Processor")
        print("=" * 60)

        top_words = self.build_vocabulary()
        self.calculate_tfidf_vectors()
        self.quantize_and_save(top_words)

        print("=" * 60)
        print("Processing completed successfully")
        print("=" * 60)

        return top_words


def main():
    DB_FOLDER = "/root/Qiyana-experiment/database"
    KEYWORD_FILE = "/root/Qiyana-experiment/keyword.txt"
    OUTPUT_FILE = "/root/Qiyana-experiment/tf-idf.txt"

    MAX_WORDS = 65536
    NUM_BINS = 1024
    MAX_DOCS = 16384

    processor = TFIDFProcessor(
        DB_FOLDER,
        KEYWORD_FILE,
        OUTPUT_FILE,
        max_words=MAX_WORDS,
        num_bins=NUM_BINS,
        chunk_size=5000,
        max_docs=MAX_DOCS
    )

    try:
        processor.process()
        print()
        print("Task completed")
        print(f"Output file: {OUTPUT_FILE}")
        print(f"Selected up to {MAX_WORDS} keywords from {len(processor.all_keywords)} unique keywords")
        print(f"Processed {processor.num_docs} documents")
        print(f"Each document vector has {len(processor.word_to_id)} dimensions, quantized to 0-{NUM_BINS - 1}")
    except Exception as e:
        print(f"Error during processing: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
