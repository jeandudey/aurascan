;;; SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
;;; SPDX-License-Identifier: GPL-3.0-or-later

(use-modules (gnu packages wine)
             (guix build-system meson)
             (guix gexp)
             (guix git-download)
             (guix packages)
             ((guix licenses) #:prefix license:)
             (guix utils))

(define-public freetrackclient64
  (package
   (name "freetrackclient64")
   (version "1.0.0.0")
   (source (local-file "wine/freetrackclient" "freetrackclient"
                       #:recursive? #t
                       #:select? (git-predicate (dirname (current-filename)))))
   (build-system meson-build-system)
   (arguments
    (list #:target "x86_64-w64-mingw32"
          ;; XXX: FTGetData returns FALSE on build environment, should at
          ; least return TRUE once with data filled with zeroes.
          #:tests? #f
          #:phases
          #~(modify-phases %standard-phases
              (add-before 'check 'create-wineprefix
                (lambda _
                  (let ((wineprefix (string-append (getcwd) "/.wine"))
                        (home (string-append (getcwd) "/.home")))
                    (mkdir-p wineprefix)
                    (mkdir-p home)
                    (setenv "HOME" home)
                    (setenv "WINEPREFIX" wineprefix)
                    (setenv "WINEDEBUG" "-all")
                    (invoke "wineboot" "--init")))))))
   (native-inputs (list wine64))
   (home-page "https://github.com/jeandudey/aurascan")
   (synopsis "FreeTrack protocol dynamic library for Windows")
   (description "This provides a @acronym{DLL, Dynamic Link Library} for the
FreeTrack protocol.")
   (license (list license:expat license:isc))))

(define-public freetrackclient
  (package
    (inherit freetrackclient64)
    (name "freetrackclient")
    (arguments
     (substitute-keyword-arguments (package-arguments freetrackclient64)
       ((#:target _ #f) "i686-w64-mingw32")))
    (native-inputs (list wine))))

(list freetrackclient
      freetrackclient64)
