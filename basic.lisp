(defun len (s)
    (if s
        (+ 1 (len (cdr s)))
        0))

(defun substr-start (s start)
    (if (= start 0)
        s
    (if (= start 1)
        (cdr s)
        (substr-start (cdr s) (- start 1)))))

(defun substr-len (s l)
    (let ((slen (len s)))
        (if (< slen l)
            s
            (reverse (substr-start (reverse s) (- slen l))))))

(defun substr (s start leng)
    (if leng
        (substr-len (substr-start s start) leng)
        (substr-start s start)))

(defun rev (l)
    (if (null l)
        nil
        (append (rev (cdr l)) (list (car l)))))

; (print (substr (coerce "abcd" 'list) 1))

(print (rev (coerce "abcd" 'list)))
