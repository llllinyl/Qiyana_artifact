import os
import re
import math
from collections import defaultdict, Counter
import numpy as np
from concurrent.futures import ProcessPoolExecutor
import multiprocessing as mp

class TFIDFProcessor:
    def __init__(self, db_folder, keyword_file, output_file, max_words=65536, num_bins=1024):
        self.db_folder = db_folder
        self.keyword_file = keyword_file
        self.output_file = output_file
        self.max_words = max_words
        self.num_bins = num_bins
        
        self.txt_files = sorted(
            [f for f in os.listdir(db_folder) if f.endswith('.txt')],
            key=lambda x: int(x.split('.')[0])
        )
        self.num_docs = len(self.txt_files)
        
        self.all_keywords = []
        self.word_to_id = {}
        self.id_to_word = {}
        self.doc_freq = defaultdict(int)
        self.doc_word_counts = []
        self.tfidf_matrix = None
        
    def load_keywords(self):
        print(f"Loading keywords from {self.keyword_file}...")
        
        with open(self.keyword_file, 'r', encoding='utf-8') as f:
            content = f.read().strip()
            keywords = [kw.strip() for kw in content.split(',') if kw.strip()]
            self.all_keywords = list(set(keywords))
        
        print(f"Loaded {len(keywords)} total entries, {len(self.all_keywords)} unique keywords")
        return self.all_keywords
    
    def clean_text_for_keywords(self, text):
        if not text:
            return []
        
        text = text.lower()
        text = re.sub(r'[^\w\s]', ' ', text)
        words = text.split()
        
        stop_words = {'a', 'an', 'the', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by'}
        meaningful_words = []
        
        for word in words:
            if len(word) > 1 and word not in stop_words:
                meaningful_words.append(word)
                
        return meaningful_words
    
    def process_document(self, file_idx, file_name, keywords_set):
        file_path = os.path.join(self.db_folder, file_name)
        
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            lines = f.readlines()
        
        word_counts = Counter()
        
        if len(lines) >= 3:
            content = ' '.join(lines[2:])
            words = self.clean_text_for_keywords(content)
            
            for word in words:
                if word in keywords_set:
                    word_counts[word] += 1
        
        return file_idx, word_counts
    
    def build_vocabulary(self, num_workers=None):
        if num_workers is None:
            num_workers = mp.cpu_count()
        
        self.load_keywords()
        
        keywords_set = set(self.all_keywords)
        print(f"Keywords set size: {len(keywords_set)}")
        
        print(f"Processing {self.num_docs} documents using {num_workers} processes...")
        
        word_doc_counts = defaultdict(int)
        self.doc_word_counts = [None] * self.num_docs
        
        with ProcessPoolExecutor(max_workers=num_workers) as executor:
            futures = []
            for i, file_name in enumerate(self.txt_files):
                futures.append(executor.submit(self.process_document, i, file_name, keywords_set))
            
            for i, future in enumerate(futures):
                doc_idx, word_counts = future.result()
                self.doc_word_counts[doc_idx] = word_counts
                
                for word in set(word_counts.keys()):
                    word_doc_counts[word] += 1
                
                if (i + 1) % 5000 == 0:
                    print(f"Processed {i + 1}/{self.num_docs} documents")
        
        print(f"Found {len(word_doc_counts)} keywords that appear in documents")
        
        idf_scores = {}
        for word, doc_count in word_doc_counts.items():
            idf_scores[word] = math.log(self.num_docs / (1 + doc_count))
        
        missing_keywords = keywords_set - set(word_doc_counts.keys())
        if missing_keywords:
            print(f"Warning: {len(missing_keywords)} keywords do not appear in any document")
            for word in missing_keywords:
                idf_scores[word] = math.log(self.num_docs / 1)
        
        sorted_keywords = sorted(idf_scores.items(), key=lambda x: x[1], reverse=True)
        top_words = sorted_keywords[:self.max_words]
        
        for idx, (word, _) in enumerate(top_words):
            self.word_to_id[word] = idx
            self.id_to_word[idx] = word
        
        self.doc_freq = {word: word_doc_counts.get(word, 0) for word in self.word_to_id.keys()}
        
        print(f"Selected {len(self.word_to_id)} keywords with highest IDF")
        print(f"IDF range: min={min(idf_scores[w] for w in self.word_to_id.keys()):.4f}, "
              f"max={max(idf_scores[w] for w in self.word_to_id.keys()):.4f}")
        
        return top_words
    
    def calculate_tfidf_matrix(self):
        num_docs = self.num_docs
        num_words = len(self.word_to_id)
        
        tfidf_matrix = np.zeros((num_docs, num_words), dtype=np.float32)
        
        print("Calculating TF-IDF matrix...")
        
        for doc_idx in range(num_docs):
            word_counts = self.doc_word_counts[doc_idx]
            total_words = sum(word_counts.values())
            
            if total_words == 0:
                continue
                
            for word, count in word_counts.items():
                if word in self.word_to_id:
                    word_id = self.word_to_id[word]
                    
                    tf = count / total_words
                    idf = math.log(num_docs / (1 + self.doc_freq[word]))
                    
                    tfidf_matrix[doc_idx, word_id] = tf * idf
            
            if (doc_idx + 1) % 5000 == 0:
                print(f"Calculated {doc_idx + 1}/{num_docs} documents")
        
        self.tfidf_matrix = tfidf_matrix
        return tfidf_matrix
    
    def quantize_matrix(self, tfidf_matrix):
        print("Quantizing matrix...")
        
        global_min = tfidf_matrix.min()
        global_max = tfidf_matrix.max()
        
        print(f"TF-IDF value range: {global_min:.6f} - {global_max:.6f}")
        
        if global_max - global_min < 1e-10:
            quantized_matrix = np.zeros_like(tfidf_matrix, dtype=np.int16)
        else:
            normalized = (tfidf_matrix - global_min) / (global_max - global_min)
            quantized_matrix = (normalized * (self.num_bins - 1)).astype(np.int16)
        
        print(f"Quantized matrix shape: {quantized_matrix.shape}")
        print(f"Value range: {quantized_matrix.min()} - {quantized_matrix.max()}")
        
        return quantized_matrix
    
    def save_results(self, top_words, quantized_matrix):
        print("Saving results to file...")
        
        with open(self.output_file, 'w', encoding='utf-8') as f:
            words = [word for word, _ in top_words]
            f.write(','.join(words) + '\n')
            
            for i in range(self.num_docs):
                row_values = quantized_matrix[i].tolist()
                row_str = ','.join(map(str, row_values))
                f.write(row_str + '\n')
                
                if (i + 1) % 10000 == 0:
                    print(f"Saved {i + 1}/{self.num_docs} rows")
        
        print(f"Results saved to {self.output_file}")
        file_size = os.path.getsize(self.output_file) / (1024 * 1024)
        print(f"File size: {file_size:.2f} MB")
    
    def process(self):
        print("=" * 60)
        print("TF-IDF Processor with Predefined Keywords")
        print("=" * 60)
        
        top_words = self.build_vocabulary()
        tfidf_matrix = self.calculate_tfidf_matrix()
        quantized_matrix = self.quantize_matrix(tfidf_matrix)
        self.save_results(top_words, quantized_matrix)
        
        print("=" * 60)
        print("Processing complete!")
        print("=" * 60)
        
        return top_words, quantized_matrix


def main():
    DB_FOLDER = "/root/Qiyana-experiment/database"
    KEYWORD_FILE = "/root/Qiyana-experiment/keyword.txt"
    OUTPUT_FILE = "/root/Qiyana-experiment/tf-idf.txt"
    MAX_WORDS = 65536
    NUM_BINS = 1024
    
    processor = TFIDFProcessor(DB_FOLDER, KEYWORD_FILE, OUTPUT_FILE, MAX_WORDS, NUM_BINS)
    
    try:
        processor.process()
        print("\nTask completed!")
        print(f"Output file: {OUTPUT_FILE}")
        print(f"Selected {MAX_WORDS} keywords from {len(processor.all_keywords)} unique keywords")
        print(f"Processed {processor.num_docs} documents")
        print(f"Each document vector has {MAX_WORDS} dimensions (quantized to 0-{NUM_BINS-1})")
    except Exception as e:
        print(f"Error during processing: {str(e)}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
