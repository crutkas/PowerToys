# PowerRename Metadata and EXIF Support

PowerRename now supports extracting and using file metadata and EXIF data in rename operations.

## Using Metadata in Rename Operations

You can use file metadata and EXIF data in your rename patterns by using special syntax:

### File Metadata
To use general file metadata, use the following syntax:
```
$file.PropertyName$
```

For example:
- `$file.Size$` - Gets the file size
- `$file.DateCreated$` - Gets the file creation date
- `$file.DateModified$` - Gets the file modification date

### EXIF Data
To use EXIF data (applicable to image files), use the following syntax:
```
$exif.PropertyName$
```

For example:
- `$exif.DateTaken$` - Gets the date when the photo was taken
- `$exif.CameraModel$` - Gets the camera model
- `$exif.CameraManufacturer$` - Gets the camera manufacturer

## Available Metadata Properties

### Common File Properties
- `System.Size` - File size
- `System.ItemType` - Type of file
- `System.DateCreated` - Date the file was created
- `System.DateModified` - Date the file was last modified
- `System.DateAccessed` - Date the file was last accessed
- `System.FileAttributes` - File attributes
- `System.ComputerName` - Name of the computer
- `System.Author` - Author of the file
- `System.Title` - Title of the file
- `System.Subject` - Subject of the file
- `System.Keywords` - Keywords associated with the file
- `System.Comment` - Comments on the file
- `System.Copyright` - Copyright information

### Music Properties
- `System.Music.AlbumTitle` - Title of the album
- `System.Music.Artist` - Artist name
- `System.Music.Genre` - Music genre

### Video Properties
- `System.Video.FrameWidth` - Width of the video in pixels
- `System.Video.FrameHeight` - Height of the video in pixels
- `System.Video.FrameRate` - Frame rate of the video

### Image Properties
- `System.Image.Dimensions` - Image dimensions
- `System.Image.HorizontalSize` - Horizontal size of the image
- `System.Image.VerticalSize` - Vertical size of the image
- `System.Image.BitDepth` - Bit depth of the image

### Document Properties
- `System.Document.PageCount` - Number of pages in the document

### EXIF Properties (for photos)
- `System.Photo.ExposureTime` - Exposure time
- `System.Photo.FNumber` - F-number
- `System.Photo.ISOSpeed` - ISO speed
- `System.Photo.ExposureBias` - Exposure bias
- `System.Photo.FocalLength` - Focal length
- `System.Photo.Flash` - Flash information
- `System.Photo.Orientation` - Orientation
- `System.Photo.MeteringMode` - Metering mode
- `System.Photo.LightSource` - Light source
- `System.Photo.DateTaken` - Date the photo was taken
- `System.Photo.CameraManufacturer` - Camera manufacturer
- `System.Photo.CameraModel` - Camera model
- `System.Photo.FocalLengthInFilm` - Focal length in film
- `System.Photo.DigitalZoom` - Digital zoom
- `System.GPS.Latitude` - GPS latitude
- `System.GPS.Longitude` - GPS longitude
- `System.GPS.Altitude` - GPS altitude

## Examples

1. Rename photos to include the date taken and camera model:
   - Search: `IMG_(\d+).jpg`
   - Replace: `Photo_$exif.DateTaken$_$exif.CameraModel$_$1.jpg`

2. Rename music files to include artist and album:
   - Search: `(\d+)_Track.mp3`
   - Replace: `$file.Music.Artist$ - $file.Music.AlbumTitle$ - $1.mp3`

3. Include file size in the filename:
   - Search: `Document.docx`
   - Replace: `Document_$file.Size$.docx`