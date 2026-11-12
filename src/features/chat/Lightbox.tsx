// The lightbox: one image of a message at full size; ← and → step through the message's images, Esc closes.
import { createPortal } from "react-dom";
import type { Image } from "../../ipc/bindings";
import { ArrowLeftIcon, ArrowRightIcon, CloseIcon } from "../../ui/icons";
import { imageUrl } from "./attachments";

type Props = { images: Image[]; index: number; onStep: (by: -1 | 1) => void; onClose: () => void };

export function Lightbox({ images, index, onStep, onClose }: Props) {
  const last = images.length - 1;
  return createPortal(
    <div className="lightbox" role="dialog" aria-modal="true" aria-label="Image" onMouseDown={onClose} onClick={(e) => e.stopPropagation()}>
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
