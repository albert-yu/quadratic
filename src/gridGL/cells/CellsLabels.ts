import {
  BLEND_MODES,
  BitmapFont,
  Container,
  Mesh,
  MeshGeometry,
  MeshMaterial,
  Point,
  Program,
  Rectangle,
  Renderer,
  Texture,
} from 'pixi.js';
import { Bounds } from '../../grid/sheet/Bounds';
import { Sheet } from '../../grid/sheet/Sheet';
import { PageMeshData } from '../pixiOverride/TextMesh';
import { msdfFrag, msdfVert } from '../pixiOverride/shader';
import { CellLabel } from './CellLabel';
import { CellsHash } from './CellsHash';
import { CellHash, CellRust } from './CellsTypes';

// holds all CellLabels within a sheet
export class CellsLabels extends Container implements CellHash {
  private cellsHash: CellsHash;
  private textureCache: Texture[] = [];

  cellLabels: CellLabel[];

  // this is used by CellsHash
  hashes: Set<CellsHash>;
  AABB?: Rectangle;

  // this is used to render all bitmapText within this region
  private finalBitmapText: Container;
  private pagesMeshData: Record<number, PageMeshData> = {};

  constructor(cellsHash: CellsHash) {
    super();
    this.cellsHash = cellsHash;
    this.cellLabels = [];
    this.hashes = new Set();
    this.finalBitmapText = this.addChild(new Container());
  }

  get sheet(): Sheet {
    return this.cellsHash.sheet;
  }

  create(cells?: CellRust[]): CellLabel[] {
    this.cellLabels = [];
    cells = cells ?? this.sheet.grid.getCellList(this.cellsHash.AABB);
    const cellLabels = cells.map((cell) => {
      const rectangle = this.sheet.gridOffsets.getCell(cell.x, cell.y);
      const cellLabel = new CellLabel(cell, rectangle);
      this.cellLabels.push(cellLabel);
      return cellLabel;
    });
    this.updateText();
    return cellLabels;
  }

  render(renderer: Renderer) {
    // if the object is not visible or the alpha is 0 then no need to render this element
    if (!this.visible || this.worldAlpha <= 0 || !this.renderable) {
      return;
    }

    // Inject the shader code with the correct value
    const { a, b, c, d } = this.transform.worldTransform;

    const dx = Math.sqrt(a * a + b * b);
    const dy = Math.sqrt(c * c + d * d);
    const worldScale = (Math.abs(dx) + Math.abs(dy)) / 2;

    const resolution = renderer.resolution;

    for (const id in this.pagesMeshData) {
      const pagesMeshData = this.pagesMeshData[id];
      const { distanceFieldRange, size } = BitmapFont.available[pagesMeshData.fontName];
      const fontScale = pagesMeshData.fontSize / size;
      pagesMeshData.mesh.shader.uniforms.uFWidth = worldScale * distanceFieldRange * fontScale * resolution;
    }
    this.finalBitmapText.render(renderer);
  }

  public updateText(): void {
    this.finalBitmapText.removeChildren();

    this.cellLabels.forEach((child) => child.updateText());
    this.pagesMeshData = {};

    this.cellLabels.forEach((cellLabel) => {
      const lenChars = cellLabel.chars.length;

      for (let i = 0; i < lenChars; i++) {
        const texture = cellLabel.chars[i].texture;
        const baseTextureUid = texture.baseTexture.uid;
        let pageMeshData = this.pagesMeshData[baseTextureUid];
        if (!pageMeshData) {
          const geometry = new MeshGeometry();
          let material: MeshMaterial;
          let meshBlendMode: BLEND_MODES;

          material = new MeshMaterial(Texture.EMPTY, {
            program: Program.from(msdfVert, msdfFrag),
            uniforms: { uFWidth: 0 },
          });
          meshBlendMode = BLEND_MODES.NORMAL_NPM;

          const mesh = new Mesh(geometry, material);
          mesh.blendMode = meshBlendMode;

          const pageMeshData = {
            fontName: cellLabel.fontName,
            fontSize: cellLabel.fontSize,
            index: 0,
            indexCount: 0,
            vertexCount: 0,
            uvsCount: 0,
            total: 0,
            mesh,
            vertices: undefined,
            uvs: undefined,
            indices: undefined,
            colors: undefined,
          };

          this.textureCache[baseTextureUid] = this.textureCache[baseTextureUid] || new Texture(texture.baseTexture);
          pageMeshData.mesh.texture = this.textureCache[baseTextureUid];
          this.pagesMeshData[baseTextureUid] = pageMeshData;
          this.finalBitmapText.addChild(pageMeshData.mesh);
        }

        this.pagesMeshData[baseTextureUid].total++;
      }
    });

    for (const id in this.pagesMeshData) {
      const pageMeshData = this.pagesMeshData[id];
      const total = pageMeshData.total;
      pageMeshData.vertices = new Float32Array(4 * 2 * total);
      pageMeshData.uvs = new Float32Array(4 * 2 * total);
      pageMeshData.indices = new Uint16Array(6 * total);
      pageMeshData.colors = new Float32Array(4 * 4 * total);
      pageMeshData.mesh.geometry.addAttribute('aColors', pageMeshData.colors, 4);

      // as a buffer maybe bigger than the current word, we set the size of the meshMaterial
      // to match the number of letters needed
      pageMeshData.mesh.size = 6 * total;
    }

    this.cellLabels.forEach((cellLabel) => cellLabel.updatePageMesh(this.pagesMeshData));

    for (const id in this.pagesMeshData) {
      const pageMeshData = this.pagesMeshData[id];

      // cellLabel.maxLineHeight = maxLineHeight * scale;

      const vertexBuffer = pageMeshData.mesh.geometry.getBuffer('aVertexPosition');
      const textureBuffer = pageMeshData.mesh.geometry.getBuffer('aTextureCoord');
      const colorBuffer = pageMeshData.mesh.geometry.getBuffer('aColors');
      const indexBuffer = pageMeshData.mesh.geometry.getIndex();

      vertexBuffer.data = pageMeshData.vertices!;
      textureBuffer.data = pageMeshData.uvs!;
      indexBuffer.data = pageMeshData.indices!;
      colorBuffer.data = pageMeshData.colors!;

      vertexBuffer.update();
      textureBuffer.update();
      indexBuffer.update();
      colorBuffer.update();
    }
  }

  private getClipRight(label: CellLabel, textWidth: number): number | undefined {
    // const rightEnd = label.x + textWidth;
    // let column = label.data.location.x + 1;
    // const row = label.data.location.y;
    // let neighborOffset = this.sheet.gridOffsets.getCell(column, row).x;
    // while (neighborOffset < rightEnd) {
    //   const cell = this.sheet.grid.get(column, row)?.cell;
    //   if (cell?.value || (cell?.evaluation_result && cell?.evaluation_result?.success === false)) {
    //     return neighborOffset;
    //   }
    //   const neighborWidth = this.sheet.gridOffsets.getColumnWidth(column);
    //   neighborOffset += neighborWidth;
    //   column++;
    // }
    return;
  }

  private getClipLeft(label: CellLabel): number | undefined {
    // const leftEnd = label.x;
    // let column = label.data.location.x - 1;
    // const row = label.data.location.y;
    // let neighbor = this.app.sheet.gridOffsets.getCell(column, row);
    // let neighborWidth = neighbor.width;
    // let neighborOffset = neighbor.x + neighbor.width;
    // while (neighborOffset > leftEnd) {
    //   const cell = this.app.sheet.grid.get(column, row)?.cell;
    //   if (cell?.value || (cell?.evaluation_result && cell?.evaluation_result?.success === false)) {
    //     return neighborOffset;
    //   }
    //   neighborOffset -= neighborWidth;
    //   column--;
    //   neighborWidth = this.app.sheet.gridOffsets.getColumnWidth(column);
    // }
    return;
  }

  // checks to see if the label needs to be clipped based on other labels
  private checkForClipping(label: CellLabel): void {
    // const data = label.data;
    // if (!data) {
    //   throw new Error('Expected label.data to be defined in checkForClipping');
    // }
    // const textWidth = label.getFullTextWidth();
    // if (textWidth > data.expectedWidth) {
    //   let clipLeft: number | undefined, clipRight: number | undefined;
    //   if (data.alignment === 'right') {
    //     clipLeft = this.getClipLeft(label);
    //   } else if (data.alignment === 'center') {
    //     clipLeft = this.getClipLeft(label);
    //     clipRight = this.getClipRight(label, textWidth);
    //   } else {
    //     clipRight = this.getClipRight(label, textWidth);
    //   }
    //   label.setClip({ clipLeft, clipRight });
    // } else {
    //   label.setClip();
    // }
  }

  private checkForOverflow(options: { label: CellLabel; bounds: Bounds }): void {
    // const { label, bounds } = options;
    // const { data } = label;
    // const { alignment } = data;
    // // track overflowed widths
    // const width = label.textWidth;
    // if (width > data.expectedWidth) {
    //   if (alignment === 'left' && !label.clipRight) {
    //     label.overflowRight = width - data.expectedWidth;
    //     label.overflowLeft = undefined;
    //   } else if (alignment === 'right' && !label.clipLeft) {
    //     label.overflowLeft = width - data.expectedWidth;
    //     label.overflowRight = undefined;
    //   } else if (alignment === 'center') {
    //     const overflow = (width - data.expectedWidth) / 2;
    //     if (!label.clipLeft) {
    //       label.overflowLeft = overflow;
    //     }
    //     if (!label.clipRight) {
    //       label.overflowRight = overflow;
    //     }
    //   }
    // } else {
    //   label.overflowRight = undefined;
    //   label.overflowLeft = undefined;
    // }
    // bounds.addRectangle(new Rectangle(label.x, label.y, width, label.height));
  }

  // todo: update AABB based on position
  private calculatePosition(label: CellLabel): Point {
    let alignment = label.alignment ?? 'left';
    if (alignment === 'right') {
      return new Point(label.topLeft.x + label.right - label.textWidth, label.topLeft.y);
    } else if (alignment === 'center') {
      return new Point(label.topLeft.x + label.right / 2 - label.textWidth / 2, label.topLeft.y);
    }
    return label.topLeft;
  }

  private updateLabel(label: CellLabel): void {
    label.visible = true;

    label.position = this.calculatePosition(label);

    // this ensures that the text is redrawn during column resize (otherwise clipping will not work properly)
    if (!label.lastPosition || !label.lastPosition.equals(label.position)) {
      label.dirty = true;
      label.lastPosition = label.position.clone();
    }
  }

  /**
   * add labels to headings using cached labels
   * @returns the visual bounds only if isQuadrant is defined (otherwise not worth the .width/.height call)
   */
  // update(): Rectangle | undefined {
  //   const bounds = new Bounds();

  //   // keep current children to use as the cache
  //   this.children.forEach((child) => (child.visible = false));

  //   const available = [...this.children] as CellLabel[];
  //   const leftovers: LabelData[] = [];

  //   // reuse existing labels that have the same text
  //   this.labelData.forEach((data) => {
  //     const index = available.findIndex((label) => this.compareLabelData(label, data));
  //     if (index === -1) {
  //       leftovers.push(data);
  //     } else {
  //       this.updateLabel(available[index], data);
  //       available.splice(index, 1);
  //     }
  //   });

  //   // use existing labels but change the text
  //   leftovers.forEach((data, i) => {
  //     if (i < available.length) {
  //       this.updateLabel(available[i], data);
  //     }

  //     // otherwise create new labels
  //     else {
  //       const label = this.addChild(new CellLabel(data));
  //       label.position = this.calculatePosition(label, data);
  //       label.lastPosition = label.position.clone();
  //     }
  //   });

  //   this.children.forEach((child) => {
  //     const label = child as CellLabel;
  //     if (label.visible) {
  //       this.checkForClipping(label);
  //       this.checkForOverflow({ label, bounds });
  //     }
  //   });

  //   if (!bounds.empty) {
  //     return bounds.toRectangle();
  //   }
  // }

  // todo: this is probably also not interesting
  get(): CellLabel[] {
    return this.cellLabels;
  }

  // todo: this is not interesting
  getVisible(): CellLabel[] {
    return this.cellLabels.filter((child) => child.visible) as CellLabel[];
  }
}
