import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import type { Image } from "../../ipc/bindings";
import { ArrowLeftIcon, ArrowRightIcon, CloseIcon } from "../../ui/icons";
import { imageUrl } from "./attachments";

type Props = { images: Image[]; index: number; onStep: (by: -1 | 1) => void; onClose: () => void };

export function Lightbox({ images, index, onStep, onClose }: Props) {
  const last = images.length - 1;
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    root.current?.focus();
    return () => before?.focus();
  }, []);
  return createPortal(
    <div ref={root} tabIndex={-1} className="lightbox" role="dialog" aria-modal="true" aria-label="Image" onMouseDown={onClose} onClick={(e) => e.stopPropagation()}>
      <img className="lightbox-image" src={imageUrl(images[index])} alt="" onMouseDown={(e) => e.stopPropagation()} />
      <button type="button" className="lightbox-control lightbox-close" title="Close" aria-label="Close" onMouseDown={(e) => e.stopPropagation()} onClick={onClose}>
        <CloseIcon />
      </button>
      {last > 0 && (
        <>
          <button
            type="button"
            className="lightbox-control lightbox-step"
            data-side="previous"
            title="Previous"
            aria-label="Previous"
            disabled={index === 0}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => onStep(-1)}
          >
            <ArrowLeftIcon />
          </button>
          <button
            type="button"
            className="lightbox-control lightbox-step"
            data-side="next"
            title="Next"
            aria-label="Next"
            disabled={index === last}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={() => onStep(1)}
          >
            <ArrowRightIcon />
          </button>
          <span className="lightbox-count">
            {index + 1} of {images.length}
          </span>
        </>
      )}
    </div>,
    document.body,
  );
}
